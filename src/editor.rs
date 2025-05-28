use std::fs;
use std::io::{self, Write};
use std::process::Command;
use anyhow::Result;
use colored::*;
use crate::sr_parser::SearchReplaceBlock;
use crate::llm;
use crate::input;
use crate::thinking;

pub fn apply_sr_block(block: &SearchReplaceBlock) -> Result<()> {
    // Read the file content
    let content = fs::read_to_string(&block.file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", block.file_path, e))?;

    // Try to replace the search content with replace content (only first occurrence)
    let new_content = content.replacen(&block.search_lines, &block.replace_lines, 1);
    
    // Check if any replacement was made
    if new_content == content {
        return Err(anyhow::anyhow!("Search content not found in file '{}'", block.file_path));
    }

    // Write the modified content back to the file
    fs::write(&block.file_path, new_content)
        .map_err(|e| anyhow::anyhow!("Failed to write file '{}': {}", block.file_path, e))?;

    Ok(())
}

async fn create_auto_commit(original_prompt: &str, modified_files: &[String]) -> Result<bool> {
    println!();
    println!("{}", "─".repeat(60).dimmed());
    println!("{}", "Creating commit...".bright_yellow());
    
    // Stage the modified files
    for file in modified_files {
        let output = Command::new("git")
            .arg("add")
            .arg(file)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to stage file '{}': {}", file, e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git add failed for '{}': {}", file, stderr));
        }
    }
    
    // Get the git diff of staged changes
    let diff_output = Command::new("git")
        .arg("diff")
        .arg("--cached")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to get git diff: {}", e))?;
    
    if !diff_output.status.success() {
        let stderr = String::from_utf8_lossy(&diff_output.stderr);
        return Err(anyhow::anyhow!("Git diff failed: {}", stderr));
    }
    
    let git_diff = String::from_utf8_lossy(&diff_output.stdout);
    
    if git_diff.trim().is_empty() {
        println!("Warning: No changes to commit (files may not have been modified)");
        return Ok(false);
    }
    
    // Generate commit message using LLM
    let commit_thinking = thinking::show_generating_commit();
    match llm::generate_commit_message(original_prompt, &git_diff).await {
        Ok(commit_message) => {
            commit_thinking.finish();
            println!("Generated commit message: \"{}\"", commit_message);
            
            // Create the commit
            let commit_output = Command::new("git")
                .arg("commit")
                .arg("-m")
                .arg(&commit_message)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to create commit: {}", e))?;
            
            if commit_output.status.success() {
                println!("Commit created successfully!");
                // Check if this is a self-modifying commit
                let is_self_modifying = modified_files.iter().any(|f| {
                    f.starts_with("src/") && f.ends_with(".rs") || 
                    f == "prompts.toml" ||
                    f == "Cargo.toml"
                });
                Ok(is_self_modifying)
            } else {
                let stderr = String::from_utf8_lossy(&commit_output.stderr);
                Err(anyhow::anyhow!("Git commit failed: {}", stderr))
            }
        }
        Err(e) => {
            commit_thinking.finish();
            println!("Warning: Failed to generate commit message: {}", e);
            println!("Creating commit with default message...");
            
            // Fallback to a simple commit message
            let fallback_message = format!("Auto-commit: {}", original_prompt);
            let commit_output = Command::new("git")
                .arg("commit")
                .arg("-m")
                .arg(&fallback_message)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to create fallback commit: {}", e))?;
            
            if commit_output.status.success() {
                println!("Fallback commit created successfully!");
                // Check if this is a self-modifying commit
                let is_self_modifying = modified_files.iter().any(|f| {
                    f.starts_with("src/") && f.ends_with(".rs") || 
                    f == "prompts.toml" ||
                    f == "Cargo.toml"
                });
                Ok(is_self_modifying)
            } else {
                let stderr = String::from_utf8_lossy(&commit_output.stderr);
                Err(anyhow::anyhow!("Fallback git commit failed: {}", stderr))
            }
        }
    }
}

pub async fn confirm_and_apply_blocks(blocks: Vec<SearchReplaceBlock>, original_prompt: &str, context_manager: &crate::context::ContextManager) -> Result<()> {
    if blocks.is_empty() {
        return Ok(());
    }

    println!("{}", "─".repeat(60).dimmed());
    println!("{} {}", "File edits:".bright_yellow().bold(), blocks.len());

    let mut apply_all = false;
    let mut quit_applying = false;
    let mut applied_files = Vec::new();

    for block in blocks.iter() {
        if quit_applying {
            break;
        }
        
        // Check if the file is in context
        let file_in_context = context_manager.is_file_in_context(&block.file_path);
        
        println!();
        let warning = if !file_in_context { " (NOT IN CONTEXT - BLOCKED)".red().bold() } else { "".normal() };
        println!("{}{}", block.file_path.bright_white().bold(), warning);
        println!("{}", "─".repeat(40).dimmed());
        
        // Display search content
        println!("{}", "Search:".dimmed());
        for line in block.search_lines.lines() {
            println!("  {}", line.red());
        }
        
        println!("{}", "Replace:".dimmed());
        for line in block.replace_lines.lines() {
            println!("  {}", line.green());
        }
        
        println!("{}", "─".repeat(40).dimmed());
        
        // Block edits to files not in context
        if !file_in_context {
            println!("{}", "❌ BLOCKED: Cannot edit file not in context!".red().bold());
            println!("{}", "To edit this file, first run:".yellow());
            println!("  {} {}", "/add_file".bright_cyan(), block.file_path.bright_white());
            println!("{} {}", "Skipped:".red(), block.file_path);
            continue;
        }
        
        // Get user confirmation unless apply_all is set
        let should_apply = if apply_all {
            true
        } else {
            loop {
                // No need for warning since we already block files not in context
                
                print!("{} ", "Apply? (y/n/a/q):".bright_white());
                io::stdout().flush()?;
                
                let choice = match input::read_single_char() {
                    Ok(c) => c.to_lowercase().to_string(),
                    Err(_) => continue,
                };
                
                match choice.as_str() {
                    "y" | "yes" => break true,
                    "n" | "no" => break false,
                    "a" | "apply_all" => {
                        apply_all = true;
                        break true;
                    },
                    "q" | "quit" => {
                        quit_applying = true;
                        break false;
                    },
                    _ => {
                        println!("Please enter 'y' (yes), 'n' (no), 'a' (apply all), or 'q' (quit)");
                        continue;
                    }
                }
            }
        };

        if should_apply {
            match apply_sr_block(block) {
                Ok(()) => {
                    println!("{} {}", "Applied:".green(), block.file_path);
                    applied_files.push(block.file_path.clone());
                }
                Err(e) => {
                    println!("{} {} - {}", "Failed:".red(), block.file_path, e);
                }
            }
        } else {
            println!("{} {}", "Skipped:".dimmed(), block.file_path);
        }
    }

    if quit_applying && blocks.len() > 1 {
        println!("\nWarning: Stopped applying changes (remaining {} changes were skipped)", 
                 blocks.len() - blocks.iter().position(|_| quit_applying).unwrap_or(0));
    }

    println!("\nFile editing session complete.");
    
    // Create automatic commit if any files were modified
    if !applied_files.is_empty() {
        match create_auto_commit(original_prompt, &applied_files).await {
            Ok(is_self_modifying) => {
                if is_self_modifying {
                    println!();
                    println!("{}", "─".repeat(60).dimmed());
                    println!("{}", "Self-modification complete!".bright_green().bold());
                    println!("KOTA needs to be rebuilt and restarted for changes to take effect.");
                    println!("Exiting with code 123 to signal restart...");
                    std::process::exit(123);
                }
            }
            Err(e) => {
                println!("Warning: Auto-commit failed: {}", e);
                println!("You can manually commit the changes with: git add . && git commit");
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_apply_sr_block_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nold content\nline3";
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "old content".to_string(),
            replace_lines: "new content".to_string(),
        };

        assert!(apply_sr_block(&block).is_ok());
        
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(new_content, "line1\nnew content\nline3");
    }

    #[test]
    fn test_apply_sr_block_not_found() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nsome content\nline3";
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "nonexistent content".to_string(),
            replace_lines: "new content".to_string(),
        };

        assert!(apply_sr_block(&block).is_err());
    }

    #[test]
    fn test_apply_sr_block_multiple_occurrences() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "duplicate\nline2\nduplicate\nline4";
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "duplicate".to_string(),
            replace_lines: "replaced".to_string(),
        };

        assert!(apply_sr_block(&block).is_ok());
        
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        // Should only replace first occurrence
        assert_eq!(new_content, "replaced\nline2\nduplicate\nline4");
    }

    #[test]
    fn test_apply_sr_block_empty_search() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nline2\nline3";
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "".to_string(),
            replace_lines: "inserted".to_string(),
        };

        assert!(apply_sr_block(&block).is_ok());
        
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        // Empty search should insert at beginning
        assert_eq!(new_content, "insertedline1\nline2\nline3");
    }

    #[test]
    fn test_apply_sr_block_empty_replace() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nto_delete\nline3";
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "to_delete".to_string(),
            replace_lines: "".to_string(),
        };

        assert!(apply_sr_block(&block).is_ok());
        
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        // Should delete the content
        assert_eq!(new_content, "line1\n\nline3");
    }

    #[test]
    fn test_apply_sr_block_multiline() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nold line 1\nold line 2\nline4";
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "old line 1\nold line 2".to_string(),
            replace_lines: "new line 1\nnew line 2\nnew line 3".to_string(),
        };

        assert!(apply_sr_block(&block).is_ok());
        
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(new_content, "line1\nnew line 1\nnew line 2\nnew line 3\nline4");
    }

    #[test]
    fn test_apply_sr_block_file_not_found() {
        let block = SearchReplaceBlock {
            file_path: "/nonexistent/file.txt".to_string(),
            search_lines: "search".to_string(),
            replace_lines: "replace".to_string(),
        };

        let result = apply_sr_block(&block);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read file"));
    }

    #[test]
    fn test_apply_sr_block_special_characters() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = r#"let regex = r"^.*\d+.*$";"#;
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: r#"r"^.*\d+.*$""#.to_string(),
            replace_lines: r#"r"^.*\w+.*$""#.to_string(),
        };

        assert!(apply_sr_block(&block).is_ok());
        
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains(r#"r"^.*\w+.*$""#));
    }

    #[test]
    fn test_apply_sr_block_whitespace_preservation() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "    function old() {\n        return \"old\";\n    }";
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "    function old() {\n        return \"old\";\n    }".to_string(),
            replace_lines: "    function new() {\n        return \"new\";\n    }".to_string(),
        };

        assert!(apply_sr_block(&block).is_ok());
        
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("function new()"));
        assert!(new_content.contains("return \"new\""));
    }

    #[test]
    fn test_apply_sr_block_substring_matching() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nsome content here\nline3";
        fs::write(temp_file.path(), content).unwrap();

        // Substring matching works - "some content" matches within "some content here"
        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "some content".to_string(),
            replace_lines: "new content".to_string(),
        };

        assert!(apply_sr_block(&block).is_ok());
        
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        // "some content" gets replaced with "new content", leaving "here"
        assert_eq!(new_content, "line1\nnew content here\nline3");
    }

    #[test]
    fn test_apply_sr_block_no_match() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nsome content here\nline3";
        fs::write(temp_file.path(), content).unwrap();

        // This should genuinely not match
        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "completely different text".to_string(),
            replace_lines: "new content".to_string(),
        };

        let result = apply_sr_block(&block);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
        
        // Verify file wasn't changed
        let unchanged_content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(unchanged_content, "line1\nsome content here\nline3");
    }
}