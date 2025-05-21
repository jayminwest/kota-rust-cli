use regex::Regex;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SearchReplaceBlock {
    pub file_path: String,
    pub search_lines: String,
    pub replace_lines: String,
}

pub fn parse_sr_blocks(response: &str) -> Result<Vec<SearchReplaceBlock>> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = response.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Look for file path followed by <<<<<<< SEARCH
        if i + 1 < lines.len() && lines[i + 1].trim() == "<<<<<<< SEARCH" {
            let file_path = lines[i].trim().to_string();
            
            // Skip the <<<<<<< SEARCH line
            i += 2;
            
            // Collect search lines until we find ======= 
            let mut search_lines = Vec::new();
            let mut found_separator = false;
            
            while i < lines.len() {
                let line = lines[i];
                if line.trim().starts_with("=======") {
                    found_separator = true;
                    i += 1;
                    break;
                }
                search_lines.push(line);
                i += 1;
            }
            
            if !found_separator {
                return Err(anyhow::anyhow!("Malformed S/R block: missing separator for file {}", file_path));
            }
            
            // Collect replace lines until we find >>>>>>> REPLACE
            let mut replace_lines = Vec::new();
            let mut found_end = false;
            
            while i < lines.len() {
                let line = lines[i];
                if line.trim() == ">>>>>>> REPLACE" {
                    found_end = true;
                    i += 1;
                    break;
                }
                replace_lines.push(line);
                i += 1;
            }
            
            if !found_end {
                return Err(anyhow::anyhow!("Malformed S/R block: missing >>>>>>> REPLACE for file {}", file_path));
            }
            
            let search_content = search_lines.join("\n");
            let replace_content = replace_lines.join("\n");
            
            blocks.push(SearchReplaceBlock {
                file_path,
                search_lines: search_content,
                replace_lines: replace_content,
            });
        } else {
            i += 1;
        }
    }
    
    Ok(blocks)
}

pub fn contains_sr_blocks(response: &str) -> bool {
    let search_pattern = Regex::new(r"<<<<<<< SEARCH").unwrap();
    search_pattern.is_match(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_sr_block() {
        let input = r#"Here's your file change:

src/main.rs
<<<<<<< SEARCH
fn old_function() {
    println!("old");
}
=======
fn new_function() {
    println!("new");
}
>>>>>>> REPLACE

Done!"#;

        let blocks = parse_sr_blocks(input).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].file_path, "src/main.rs");
        assert!(blocks[0].search_lines.contains("old_function"));
        assert!(blocks[0].replace_lines.contains("new_function"));
    }

    #[test]
    fn test_multiple_sr_blocks() {
        let input = r#"
src/file1.rs
<<<<<<< SEARCH
old content 1
=======
new content 1
>>>>>>> REPLACE

src/file2.rs
<<<<<<< SEARCH
old content 2
=======
new content 2
>>>>>>> REPLACE
"#;

        let blocks = parse_sr_blocks(input).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].file_path, "src/file1.rs");
        assert_eq!(blocks[1].file_path, "src/file2.rs");
    }

    #[test]
    fn test_contains_sr_blocks() {
        assert!(contains_sr_blocks("some text <<<<<<< SEARCH more text"));
        assert!(!contains_sr_blocks("regular text without markers"));
    }
}