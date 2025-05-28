use std::fs;
use anyhow::Context;
use colored::*;

pub struct ContextManager {
    items: Vec<String>,
    file_paths: Vec<String>, // Track added file paths
}

impl ContextManager {
    pub fn new() -> Self {
        Self { 
            items: Vec::new(),
            file_paths: Vec::new(),
        }
    }

    pub fn add_file(&mut self, file_path: &str) -> anyhow::Result<()> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path))?;
        self.items.push(format!("--- File: {} ---\n{}\n--- End File: {} ---", file_path, content, file_path));
        
        // Track the file path
        self.file_paths.push(file_path.to_string());
        
        println!("{} {}", "Added to context:".green(), file_path);
        Ok(())
    }

    pub fn add_snippet(&mut self, snippet: String) {
        self.items.push(format!("--- Snippet --- \n{}\n--- End Snippet ---", snippet));
        println!("{}", "Added snippet to context".green());
    }

    pub fn show_context(&self) {
        if self.items.is_empty() {
            println!("Context is empty.");
        } else {
            println!("--- Current Context ---");
            for (i, item) in self.items.iter().enumerate() {
                println!("\n[Item {}]\n{}", i + 1, item);
            }
            println!("--- End Context ---");
        }
    }

    pub fn clear_context(&mut self) {
        self.items.clear();
        self.file_paths.clear();
        println!("{}", "Context cleared".green());
    }
    
    pub fn is_file_in_context(&self, file_path: &str) -> bool {
        self.file_paths.contains(&file_path.to_string())
    }

    pub fn get_formatted_context(&self) -> String {
        let mut full_context = String::new();
        
        // Always include the list of accessible files
        if !self.file_paths.is_empty() {
            full_context.push_str("Files currently in context (you have read access to these files):\n");
            for file_path in &self.file_paths {
                full_context.push_str(&format!("- {}\n", file_path));
            }
            full_context.push_str("\nIMPORTANT: You can only suggest edits to files listed above. If you need to edit a file not in this list, tell the user to run: /add_file <filename>\n\n");
        } else {
            full_context.push_str("No files in context. To edit files, the user must first add them with: /add_file <filename>\n\n");
        }
        
        // Add the actual context items
        if !self.items.is_empty() {
            full_context.push_str("Relevant context:\n");
            for item in &self.items {
                full_context.push_str(item);
                full_context.push_str("\n\n");
            }
        }
        
        full_context
    }
    
}

