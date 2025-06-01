#![allow(dead_code)]

use std::path::{Path, PathBuf};
use anyhow::{Result, Context, bail};
use serde::{Deserialize, Serialize};

use crate::security::{ExecutionPolicy, ApprovalMode, SandboxProfile};
use crate::llm::LlmProvider;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KotaConfig {
    /// General settings
    pub general: GeneralConfig,
    
    /// LLM configuration
    pub llm: LlmConfig,
    
    /// Security settings
    pub security: SecurityConfig,
    
    /// TUI settings
    pub tui: TuiConfig,
    
    /// MCP server connections
    pub mcp: Option<McpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Enable debug mode
    pub debug: bool,
    
    /// Log level
    pub log_level: String,
    
    /// Session directory
    pub session_dir: PathBuf,
    
    /// Context directory
    pub context_dir: PathBuf,
    
    /// Maximum context size in tokens
    pub max_context_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default provider
    pub default_provider: LlmProvider,
    
    /// Provider-specific settings
    pub providers: ProvidersConfig,
    
    /// Timeout in seconds
    pub timeout_seconds: u64,
    
    /// Retry attempts
    pub retry_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub gemini: Option<GeminiConfig>,
    pub ollama: Option<OllamaConfig>,
    pub anthropic: Option<AnthropicConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Approval mode
    pub approval_mode: ApprovalMode,
    
    /// Active security policy
    pub active_policy: String,
    
    /// Custom policies
    pub policies: Vec<ExecutionPolicy>,
    
    /// Default sandbox profile
    pub default_sandbox: String,
    
    /// Custom sandbox profiles
    pub sandbox_profiles: Vec<SandboxProfile>,
    
    /// Auto-approve patterns
    pub auto_approve_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Enable TUI by default
    pub enabled: bool,
    
    /// Color scheme
    pub theme: String,
    
    /// Show file browser by default
    pub show_file_browser: bool,
    
    /// Auto-scroll chat
    pub auto_scroll: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// MCP server connections
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub capabilities: Vec<String>,
}

impl KotaConfig {
    /// Load configuration from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let mut config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        // Load API keys from environment if not in config
        config.load_env_vars();
        
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create a copy without sensitive data
        let mut safe_config = self.clone();
        safe_config.sanitize_for_save();
        
        let content = toml::to_string_pretty(&safe_config)
            .context("Failed to serialize config")?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
        
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        Ok(())
    }
    
    /// Get the default configuration path
    pub fn default_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(".kota").join("config.toml"))
    }
    
    /// Create default configuration
    pub fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let kota_dir = home.join(".kota");
        
        Self {
            general: GeneralConfig {
                debug: false,
                log_level: "info".to_string(),
                session_dir: kota_dir.join("sessions"),
                context_dir: kota_dir.join("context"),
                max_context_tokens: 100_000,
            },
            llm: LlmConfig {
                default_provider: LlmProvider::Anthropic,
                providers: ProvidersConfig {
                    gemini: Some(GeminiConfig {
                        api_key: None,
                        model: "gemini-1.5-pro-latest".to_string(),
                        temperature: 0.7,
                        max_tokens: 8192,
                    }),
                    ollama: Some(OllamaConfig {
                        base_url: "http://localhost:11434".to_string(),
                        model: "llama2".to_string(),
                        temperature: 0.7,
                    }),
                    anthropic: Some(AnthropicConfig {
                        api_key: None,
                        model: "claude-sonnet-4-20250514".to_string(),
                        temperature: 0.7,
                        max_tokens: 4096,
                    }),
                },
                timeout_seconds: 120,
                retry_attempts: 3,
            },
            security: SecurityConfig {
                approval_mode: ApprovalMode::Policy,
                active_policy: "default".to_string(),
                policies: vec![],
                default_sandbox: "development".to_string(),
                sandbox_profiles: vec![],
                auto_approve_patterns: vec![
                    "^ls".to_string(),
                    "^pwd$".to_string(),
                    "^echo".to_string(),
                ],
            },
            tui: TuiConfig {
                enabled: true,
                theme: "default".to_string(),
                show_file_browser: true,
                auto_scroll: true,
            },
            mcp: None,
        }
    }
    
    /// Load environment variables
    fn load_env_vars(&mut self) {
        // Gemini API key
        if let Some(gemini) = &mut self.llm.providers.gemini {
            if gemini.api_key.is_none() {
                gemini.api_key = std::env::var("GEMINI_API_KEY").ok();
            }
        }
        
        // Anthropic API key
        if let Some(anthropic) = &mut self.llm.providers.anthropic {
            if anthropic.api_key.is_none() {
                anthropic.api_key = std::env::var("ANTHROPIC_API_KEY").ok();
            }
        }
    }
    
    /// Remove sensitive data before saving
    fn sanitize_for_save(&mut self) {
        // Remove API keys
        if let Some(gemini) = &mut self.llm.providers.gemini {
            gemini.api_key = None;
        }
        if let Some(anthropic) = &mut self.llm.providers.anthropic {
            anthropic.api_key = None;
        }
        
        // Remove MCP API keys
        if let Some(mcp) = &mut self.mcp {
            for server in &mut mcp.servers {
                server.api_key = None;
            }
        }
    }
    
    /// Merge with command-line overrides
    pub fn merge_overrides(&mut self, overrides: Vec<(String, String)>) -> Result<()> {
        for (key, value) in overrides {
            match key.as_str() {
                "debug" => self.general.debug = value.parse()?,
                "log_level" => self.general.log_level = value,
                "provider" => self.llm.default_provider = value.parse()?,
                "approval" => self.security.approval_mode = value.parse()?,
                "theme" => self.tui.theme = value,
                _ => bail!("Unknown config key: {}", key),
            }
        }
        Ok(())
    }
}

/// Load or create configuration
pub fn load_or_create_config(path: Option<&Path>) -> Result<KotaConfig> {
    let config_path = if let Some(p) = path {
        p.to_path_buf()
    } else {
        KotaConfig::default_path()?
    };
    
    if config_path.exists() {
        KotaConfig::load(&config_path)
    } else {
        let config = KotaConfig::default();
        config.save(&config_path)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_default_config() {
        let config = KotaConfig::default();
        assert_eq!(config.general.debug, false);
        assert_eq!(config.llm.default_provider, LlmProvider::Anthropic);
        assert_eq!(config.security.approval_mode, ApprovalMode::Policy);
    }
    
    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config = KotaConfig::default();
        config.save(&config_path).unwrap();
        
        let loaded = KotaConfig::load(&config_path).unwrap();
        assert_eq!(loaded.general.debug, config.general.debug);
    }
    
    #[test]
    fn test_merge_overrides() {
        let mut config = KotaConfig::default();
        
        let overrides = vec![
            ("debug".to_string(), "true".to_string()),
            ("log_level".to_string(), "debug".to_string()),
        ];
        
        config.merge_overrides(overrides).unwrap();
        assert_eq!(config.general.debug, true);
        assert_eq!(config.general.log_level, "debug");
    }
}