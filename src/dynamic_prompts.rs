use chrono::{DateTime, Local};

use crate::context::ContextManager;

pub struct DynamicPromptData {
    pub date: String,
    pub time: String,
    pub context_file_count: usize,
    pub working_directory: String,
    pub git_branch: Option<String>,
    pub system_info: SystemInfo,
}

#[derive(Clone)]
pub struct SystemInfo {
    pub username: String,
}

impl DynamicPromptData {
    pub fn new(context_manager: &ContextManager) -> Self {
        let now: DateTime<Local> = Local::now();

        // Get git branch if in a git repo
        let git_branch = get_git_branch();

        // Get system info
        let system_info = SystemInfo {
            username: whoami::username(),
        };

        Self {
            date: now.format("%Y-%m-%d").to_string(),
            time: now.format("%H:%M:%S").to_string(),
            context_file_count: context_manager.file_paths.len(),
            working_directory: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            git_branch,
            system_info,
        }
    }
}

fn get_git_branch() -> Option<String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

// Add these dependencies to Cargo.toml:
// hostname = "0.4"
// whoami = "1.5"
