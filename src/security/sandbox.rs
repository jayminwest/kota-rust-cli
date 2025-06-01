// macOS Seatbelt sandboxing for secure command execution

use std::process::{Command, Stdio};
use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

/// macOS Seatbelt sandbox profile for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxProfile {
    /// Name of the profile
    pub name: String,
    /// Allowed read paths
    pub allowed_reads: Vec<PathBuf>,
    /// Allowed write paths
    pub allowed_writes: Vec<PathBuf>,
    /// Allow network access
    pub allow_network: bool,
    /// Allow process execution
    pub allow_exec: bool,
    /// Custom sandbox rules (raw Seatbelt syntax)
    pub custom_rules: Vec<String>,
}

impl SandboxProfile {
    /// Create a minimal sandbox profile
    pub fn minimal() -> Self {
        Self {
            name: "minimal".to_string(),
            allowed_reads: vec![],
            allowed_writes: vec![],
            allow_network: false,
            allow_exec: false,
            custom_rules: vec![],
        }
    }
    
    /// Create a read-only profile for the current directory
    pub fn read_only_current() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            name: "read_only_current".to_string(),
            allowed_reads: vec![current_dir],
            allowed_writes: vec![],
            allow_network: false,
            allow_exec: false,
            custom_rules: vec![],
        }
    }
    
    /// Create a development profile with more permissions
    pub fn development() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let temp_dir = std::env::temp_dir();
        
        Self {
            name: "development".to_string(),
            allowed_reads: vec![
                current_dir.clone(),
                PathBuf::from("/usr/bin"),
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/homebrew/bin"),
            ],
            allowed_writes: vec![
                current_dir,
                temp_dir,
            ],
            allow_network: false,
            allow_exec: true,
            custom_rules: vec![],
        }
    }
    
    /// Generate the Seatbelt profile string
    pub fn generate_profile(&self) -> String {
        let mut profile = String::from("(version 1)\n");
        profile.push_str("(deny default)\n");
        
        // Basic permissions
        profile.push_str("(allow signal)\n");
        profile.push_str("(allow system-socket)\n");
        profile.push_str("(allow file-read-metadata)\n");
        
        // Process execution
        if self.allow_exec {
            profile.push_str("(allow process-exec)\n");
            profile.push_str("(allow process-fork)\n");
        }
        
        // Network access
        if self.allow_network {
            profile.push_str("(allow network-outbound)\n");
            profile.push_str("(allow network-bind)\n");
        }
        
        // File read permissions
        for path in &self.allowed_reads {
            let path_str = path.to_string_lossy();
            profile.push_str(&format!(
                "(allow file-read* (subpath \"{}\"))\n",
                path_str
            ));
        }
        
        // File write permissions
        for path in &self.allowed_writes {
            let path_str = path.to_string_lossy();
            profile.push_str(&format!(
                "(allow file-write* (subpath \"{}\"))\n",
                path_str
            ));
        }
        
        // Custom rules
        for rule in &self.custom_rules {
            profile.push_str(rule);
            profile.push('\n');
        }
        
        profile
    }
}

/// A command that will be executed in a sandbox
pub struct SandboxedCommand {
    command: String,
    args: Vec<String>,
    profile: SandboxProfile,
}

impl SandboxedCommand {
    pub fn new(command: String, args: Vec<String>, profile: SandboxProfile) -> Self {
        Self { command, args, profile }
    }
    
    /// Execute the command in a sandbox using sandbox-exec
    pub async fn execute(&self) -> Result<std::process::Output> {
        // Generate the sandbox profile
        let profile_content = self.profile.generate_profile();
        
        // Create a temporary file for the profile
        let temp_file = tempfile::NamedTempFile::new()
            .context("Failed to create temporary profile file")?;
        
        std::fs::write(temp_file.path(), &profile_content)
            .context("Failed to write sandbox profile")?;
        
        // Build the sandbox-exec command
        let mut cmd = Command::new("sandbox-exec");
        cmd.arg("-f")
            .arg(temp_file.path())
            .arg(&self.command)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // Execute the command
        let output = tokio::task::spawn_blocking(move || {
            cmd.output()
        })
        .await
        .context("Failed to spawn blocking task")?
        .context("Failed to execute sandboxed command")?;
        
        Ok(output)
    }
    
    /// Execute with real-time output streaming
    pub async fn execute_streaming<F>(&self, on_output: F) -> Result<i32>
    where
        F: FnMut(&str) + Send + 'static,
    {
        let profile_content = self.profile.generate_profile();
        let temp_file = tempfile::NamedTempFile::new()
            .context("Failed to create temporary profile file")?;
        
        std::fs::write(temp_file.path(), &profile_content)
            .context("Failed to write sandbox profile")?;
        
        let mut cmd = Command::new("sandbox-exec");
        cmd.arg("-f")
            .arg(temp_file.path())
            .arg(&self.command)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .context("Failed to spawn sandboxed command")?;
        
        // Stream output
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        
        use std::io::{BufRead, BufReader};
        use std::sync::Arc;
        use tokio::sync::Mutex;
        
        let on_output = Arc::new(Mutex::new(on_output));
        let on_output_clone = on_output.clone();
        
        // Read stdout
        let stdout_handle = tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let on_output = on_output_clone.clone();
                tokio::runtime::Handle::current().block_on(async move {
                    let mut callback = on_output.lock().await;
                    callback(&line);
                });
            }
        });
        
        // Read stderr
        let stderr_handle = tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let on_output = on_output.clone();
                tokio::runtime::Handle::current().block_on(async move {
                    let mut callback = on_output.lock().await;
                    callback(&format!("[stderr] {}", line));
                });
            }
        });
        
        // Wait for the process to complete
        let status = tokio::task::spawn_blocking(move || {
            child.wait()
        })
        .await
        .context("Failed to wait for sandboxed command")?
        .context("Failed to get command status")?;
        
        // Wait for output handlers to complete
        let _ = tokio::join!(stdout_handle, stderr_handle);
        
        Ok(status.code().unwrap_or(-1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_minimal_profile() {
        let profile = SandboxProfile::minimal();
        let generated = profile.generate_profile();
        
        assert!(generated.contains("(deny default)"));
        assert!(generated.contains("(allow signal)"));
        assert!(!generated.contains("(allow network-outbound)"));
        assert!(!generated.contains("(allow process-exec)"));
    }
    
    #[test]
    fn test_development_profile() {
        let profile = SandboxProfile::development();
        let generated = profile.generate_profile();
        
        assert!(generated.contains("(allow process-exec)"));
        assert!(generated.contains("(allow file-write*"));
    }
    
    #[tokio::test]
    #[ignore = "sandbox-exec requires special permissions and may not work in all environments"]
    async fn test_sandboxed_command() {
        let profile = SandboxProfile::read_only_current();
        let cmd = SandboxedCommand::new(
            "echo".to_string(),
            vec!["Hello from sandbox".to_string()],
            profile,
        );
        
        let output = cmd.execute().await.unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Hello from sandbox"));
    }
}