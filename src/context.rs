use std::fs;
use anyhow::Context;

pub struct ContextManager {
    items: Vec<String>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_file(&mut self, file_path: &str) -> anyhow::Result<()> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path))?;
        self.items.push(format!("--- File: {} ---\n{}\n--- End File: {} ---", file_path, content, file_path));
        println!("Added file '{}' to context.", file_path);
        Ok(())
    }

    pub fn add_snippet(&mut self, snippet: String) {
        self.items.push(format!("--- Snippet --- \n{}\n--- End Snippet ---", snippet));
        println!("Added snippet to context.");
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
        println!("Context cleared.");
    }

    pub fn get_formatted_context(&self) -> String {
        if self.items.is_empty() {
            String::new()
        } else {
            let mut full_context = String::from("Relevant context:\n");
            for item in &self.items {
                full_context.push_str(item);
                full_context.push_str("\n\n");
            } full_context
        }
    }
}

