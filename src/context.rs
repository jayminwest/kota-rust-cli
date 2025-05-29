use std::fs;
use std::path::Path;
use anyhow::Context;
use colored::*;

pub struct ContextManager {
    items: Vec<String>,
    file_paths: Vec<String>, // Track added file paths
}

impl ContextManager {
    pub fn new() -> Self {
        let mut context = Self { 
            items: Vec::new(),
            file_paths: Vec::new(),
        };
        
        // Auto-load prompts directory if it exists
        if let Err(e) = context.load_prompts_directory() {
            eprintln!("Warning: Failed to load prompts directory: {}", e);
        }
        
        context
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
    
    /// Automatically load all prompt files from the prompts directory
    fn load_prompts_directory(&mut self) -> anyhow::Result<()> {
        let prompts_dir = Path::new("prompts");
        
        // Check if prompts directory exists
        if !prompts_dir.exists() {
            return Ok(()); // Not an error if directory doesn't exist
        }
        
        // Read all .toml files in the prompts directory
        let entries = fs::read_dir(prompts_dir)
            .with_context(|| "Failed to read prompts directory")?;
        
        let mut loaded_files = Vec::new();
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Only process .toml files
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let file_path_str = path.to_str().unwrap_or_default();
                
                // Read the file content
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read prompt file: {}", file_path_str))?;
                
                // Add to context as a prompt file
                self.items.push(format!("--- Prompt File: {} ---\n{}\n--- End Prompt File: {} ---", 
                    file_path_str, content, file_path_str));
                
                // Don't track prompt files in file_paths as they shouldn't be edited
                // Instead, just note that we loaded them
                loaded_files.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
        
        if !loaded_files.is_empty() {
            println!("{} Loaded {} prompt files: {}", 
                "Auto-loaded prompts:".green(), 
                loaded_files.len(),
                loaded_files.join(", "));
        }
        
        Ok(())
    }
    
}

