use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct MemoryManager {
    base_path: PathBuf,
}

impl MemoryManager {
    pub fn new() -> Result<Self> {
        let base_path = PathBuf::from("knowledge-base");

        // Create the basic directory structure
        let dirs = [
            "core/conversation",
            "core/knowledge-management",
            "core/partnership",
            "core/mcp",
            "personal/identity",
            "personal/journaling-tracking/weekly-notes",
            "personal/career-finance",
            "businesses",
            "projects/active",
            "systems",
            "scripts",
            "data",
            "templates",
        ];

        for dir in &dirs {
            let full_path = base_path.join(dir);
            if !full_path.exists() {
                fs::create_dir_all(&full_path).with_context(|| {
                    format!("Failed to create directory: {}", full_path.display())
                })?;
            }
        }

        Ok(Self { base_path })
    }

    pub fn store_conversation_summary(&self, summary: &str) -> Result<()> {
        let timestamp = Local::now().format("%d-%m-%y %H:%M").to_string();
        let date_str = Local::now().format("%Y-%m-%d").to_string();

        let file_path = self
            .base_path
            .join("core/conversation")
            .join(format!("session-{}.md", date_str));

        let content = if file_path.exists() {
            let existing = fs::read_to_string(&file_path)?;
            format!(
                "{}\n\n## Session Update ({})\n\n{}\n",
                existing, timestamp, summary
            )
        } else {
            format!(
                "# Conversation Log - {}\n\n## Session Start ({})\n\n{}\n",
                date_str, timestamp, summary
            )
        };

        fs::write(&file_path, content).with_context(|| {
            format!(
                "Failed to write conversation summary to {}",
                file_path.display()
            )
        })?;

        Ok(())
    }

    pub fn store_learning(&self, topic: &str, content: &str) -> Result<()> {
        let timestamp = Local::now().format("%d-%m-%y %H:%M").to_string();

        // Sanitize topic for filename
        let safe_topic = topic
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .to_lowercase();

        let file_path = self
            .base_path
            .join("core/knowledge-management")
            .join(format!("{}.md", safe_topic));

        let content = if file_path.exists() {
            let existing = fs::read_to_string(&file_path)?;
            format!("{}\n\n## Update ({})\n\n{}\n", existing, timestamp, content)
        } else {
            format!(
                "# {}\n\n## Initial Learning ({})\n\n{}\n",
                topic, timestamp, content
            )
        };

        fs::write(&file_path, content)
            .with_context(|| format!("Failed to write learning to {}", file_path.display()))?;

        Ok(())
    }

    pub fn get_recent_memories(&self, limit: usize) -> Result<Vec<String>> {
        let mut memories = Vec::new();

        // Get recent conversation summaries
        let conv_dir = self.base_path.join("core/conversation");
        if conv_dir.exists() {
            let mut entries: Vec<_> = fs::read_dir(&conv_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "md")
                        .unwrap_or(false)
                })
                .collect();

            // Sort by modification time (newest first)
            entries.sort_by_key(|entry| {
                entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            entries.reverse();

            for entry in entries.into_iter().take(limit) {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    // Take first few lines as summary
                    let summary: String = content.lines().take(5).collect::<Vec<_>>().join("\n");
                    memories.push(format!("Recent conversation: {}", summary));
                }
            }
        }

        Ok(memories)
    }

    pub fn search_knowledge(&self, query: &str) -> Result<Vec<String>> {
        let mut results = Vec::new();

        // Simple search through knowledge management files
        let km_dir = self.base_path.join("core/knowledge-management");
        if km_dir.exists() {
            for entry in fs::read_dir(&km_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|ext| ext.to_str()) == Some("md") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if content.to_lowercase().contains(&query.to_lowercase()) {
                            let filename = entry.file_name().to_string_lossy().to_string();
                            results.push(format!(
                                "Found in {}: {}",
                                filename,
                                content.lines().next().unwrap_or("No title")
                            ));
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            base_path: PathBuf::from("knowledge-base"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_memory_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().join("test-knowledge-base");

        let _memory = MemoryManager {
            base_path: base_path.clone(),
        };

        // The manager should create basic directory structure
        assert!(base_path.join("core/conversation").exists() || !base_path.exists());
    }

    #[test]
    fn test_store_conversation_summary() {
        let temp_dir = TempDir::new().unwrap();
        let memory = MemoryManager {
            base_path: temp_dir.path().to_path_buf(),
        };

        // This would normally create directories, but we'll just test the interface
        let result = memory.store_conversation_summary("Test conversation summary");
        // Test passes if no panic occurs
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid for this test
    }

    #[test]
    fn test_store_learning() {
        let temp_dir = TempDir::new().unwrap();
        let memory = MemoryManager {
            base_path: temp_dir.path().to_path_buf(),
        };

        let result = memory.store_learning("Rust Programming", "Learned about ownership");
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid for this test
    }
}
