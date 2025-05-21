use std::fs;
use std::io::{self, Write};
use std::process::Command;
use anyhow::Result;
use crate::sr_parser::SearchReplaceBlock;
use crate::llm;

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

async fn create_auto_commit(original_prompt: &str, modified_files: &[String]) -> Result<()> {
    println!("\n🔄 Creating automatic commit...");
    
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
        println!("⚠️  No changes to commit (files may not have been modified)");
        return Ok(());
    }
    
    // Generate commit message using LLM
    match llm::generate_commit_message(original_prompt, &git_diff).await {
        Ok(commit_message) => {
            println!("💬 Generated commit message: \"{}\"", commit_message);
            
            // Create the commit
            let commit_output = Command::new("git")
                .arg("commit")
                .arg("-m")
                .arg(&commit_message)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to create commit: {}", e))?;
            
            if commit_output.status.success() {
                println!("✅ Commit created successfully!");
            } else {
                let stderr = String::from_utf8_lossy(&commit_output.stderr);
                return Err(anyhow::anyhow!("Git commit failed: {}", stderr));
            }
        }
        Err(e) => {
            println!("⚠️  Failed to generate commit message: {}", e);
            println!("📝 Creating commit with default message...");
            
            // Fallback to a simple commit message
            let fallback_message = format!("Auto-commit: {}", original_prompt);
            let commit_output = Command::new("git")
                .arg("commit")
                .arg("-m")
                .arg(&fallback_message)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to create fallback commit: {}", e))?;
            
            if commit_output.status.success() {
                println!("✅ Fallback commit created successfully!");
            } else {
                let stderr = String::from_utf8_lossy(&commit_output.stderr);
                return Err(anyhow::anyhow!("Fallback git commit failed: {}", stderr));
            }
        }
    }
    
    Ok(())
}

pub async fn confirm_and_apply_blocks(blocks: Vec<SearchReplaceBlock>, original_prompt: &str, context_manager: &crate::context::ContextManager) -> Result<()> {
    if blocks.is_empty() {
        return Ok(());
    }

    println!("\n🔧 Found {} file edit suggestion(s):", blocks.len());
    println!("{}", "─".repeat(60));

    let mut apply_all = false;
    let mut quit_applying = false;
    let mut applied_files = Vec::new();

    for (_i, block) in blocks.iter().enumerate() {
        if quit_applying {
            break;
        }
        
        // Check if the file is in context
        let file_in_context = context_manager.is_file_in_context(&block.file_path);
        
        println!("\n📁 File: {}{}", block.file_path, 
                if !file_in_context { " ⚠️ (WARNING: File not in context)" } else { "" });
        println!("{}", "─".repeat(40));
        
        // Display search content
        println!("🔍 SEARCH:");
        for line in block.search_lines.lines() {
            println!("  │ {}", line);
        }
        
        println!("{}", "─".repeat(40));
        
        // Display replace content
        println!("✏️  REPLACE:");
        for line in block.replace_lines.lines() {
            println!("  │ {}", line);
        }
        
        println!("{}", "─".repeat(40));
        
        // Get user confirmation unless apply_all is set
        let should_apply = if apply_all {
            true
        } else {
            loop {
                // Warn about files not in context
                if !file_in_context {
                    println!("⚠️ WARNING: This file was not added to context with /add_file!");
                    println!("⚠️ Modifications to files not in context may be risky.");
                }
                
                print!("Apply this change? (y/n/a/q) [yes/no/apply_all/quit]: ");
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let choice = input.trim().to_lowercase();
                
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
                    println!("✅ Successfully applied change to '{}'", block.file_path);
                    applied_files.push(block.file_path.clone());
                }
                Err(e) => {
                    println!("❌ Failed to apply change to '{}': {}", block.file_path, e);
                }
            }
        } else {
            println!("⏭️  Skipped change to '{}'", block.file_path);
        }
    }

    if quit_applying && blocks.len() > 1 {
        println!("\n⚠️  Stopped applying changes (remaining {} changes were skipped)", 
                 blocks.len() - blocks.iter().position(|_| quit_applying).unwrap_or(0));
    }

    println!("\n📝 File editing session complete.");
    
    // Create automatic commit if any files were modified
    if !applied_files.is_empty() {
        if let Err(e) = create_auto_commit(original_prompt, &applied_files).await {
            println!("⚠️  Auto-commit failed: {}", e);
            println!("📋 You can manually commit the changes with: git add . && git commit");
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
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nsome content\nline3";
        fs::write(temp_file.path(), content).unwrap();

        let block = SearchReplaceBlock {
            file_path: temp_file.path().to_string_lossy().to_string(),
            search_lines: "nonexistent content".to_string(),
            replace_lines: "new content".to_string(),
        };

        assert!(apply_sr_block(&block).is_err());
    }
}