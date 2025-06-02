use anyhow::Result;
use regex::Regex;

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
                return Err(anyhow::anyhow!(
                    "Malformed S/R block: missing separator for file {}",
                    file_path
                ));
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
                return Err(anyhow::anyhow!(
                    "Malformed S/R block: missing >>>>>>> REPLACE for file {}",
                    file_path
                ));
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

    #[test]
    fn test_malformed_blocks() {
        // Missing separator
        let missing_separator = r#"
src/test.rs
<<<<<<< SEARCH
old content
>>>>>>> REPLACE
"#;
        let result = parse_sr_blocks(missing_separator);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing separator"));

        // Missing end marker
        let missing_end = r#"
src/test.rs
<<<<<<< SEARCH
old content
=======
new content
"#;
        let result = parse_sr_blocks(missing_end);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing >>>>>>> REPLACE"));

        // Partial block at end
        let partial_block = r#"
src/test.rs
<<<<<<< SEARCH
"#;
        let result = parse_sr_blocks(partial_block);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_content() {
        // Empty search content
        let empty_search = r#"
src/test.rs
<<<<<<< SEARCH
=======
new content
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(empty_search).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].search_lines, "");
        assert_eq!(blocks[0].replace_lines, "new content");

        // Empty replace content (deletion)
        let empty_replace = r#"
src/test.rs
<<<<<<< SEARCH
old content
=======
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(empty_replace).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].search_lines, "old content");
        assert_eq!(blocks[0].replace_lines, "");

        // Both empty
        let both_empty = r#"
src/test.rs
<<<<<<< SEARCH
=======
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(both_empty).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].search_lines, "");
        assert_eq!(blocks[0].replace_lines, "");
    }

    #[test]
    fn test_whitespace_handling() {
        // Extra whitespace in markers
        let whitespace_markers = r#"
src/test.rs
<<<<<<< SEARCH   
old content
=======   
new content
>>>>>>> REPLACE   
"#;
        let blocks = parse_sr_blocks(whitespace_markers).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].search_lines, "old content");
        assert_eq!(blocks[0].replace_lines, "new content");

        // Preserve indentation in content
        let indented_content = r#"
src/test.rs
<<<<<<< SEARCH
    def old_function():
        return "old"
=======
    def new_function():
        return "new"
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(indented_content).unwrap();
        assert!(blocks[0].search_lines.contains("    def old_function():"));
        assert!(blocks[0].replace_lines.contains("    def new_function():"));
    }

    #[test]
    fn test_special_characters() {
        // Content with special regex characters
        let special_content = r#"
src/test.rs
<<<<<<< SEARCH
let regex = r"^.*\d+.*$";
=======
let regex = r"^.*\w+.*$";
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(special_content).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].search_lines.contains(r#"r"^.*\d+.*$""#));
        assert!(blocks[0].replace_lines.contains(r#"r"^.*\w+.*$""#));

        // Content with S/R markers inside strings
        let embedded_markers = r#"
src/test.rs
<<<<<<< SEARCH
println!("This contains <<<<<<< SEARCH in string");
=======
println!("This contains >>>>>>> REPLACE in string");
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(embedded_markers).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].search_lines.contains("<<<<<<< SEARCH in string"));
        assert!(blocks[0]
            .replace_lines
            .contains(">>>>>>> REPLACE in string"));
    }

    #[test]
    fn test_multiline_content() {
        let multiline = r#"
src/complex.rs
<<<<<<< SEARCH
fn complex_function() {
    let data = vec![
        "item1",
        "item2",
        "item3",
    ];
    
    for item in data {
        println!("{}", item);
    }
}
=======
fn complex_function() {
    let data = vec![
        "item1",
        "item2", 
        "item3",
        "item4",
    ];
    
    for (i, item) in data.iter().enumerate() {
        println!("{}: {}", i, item);
    }
}
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(multiline).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].search_lines.contains("vec!["));
        assert!(blocks[0].replace_lines.contains("enumerate()"));
        assert!(blocks[0].search_lines.matches('\n').count() > 5);
        assert!(blocks[0].replace_lines.matches('\n').count() > 5);
    }

    #[test]
    fn test_file_path_variations() {
        // Absolute path
        let absolute_path = r#"
/home/user/project/src/main.rs
<<<<<<< SEARCH
old
=======
new
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(absolute_path).unwrap();
        assert_eq!(blocks[0].file_path, "/home/user/project/src/main.rs");

        // Windows path
        let windows_path = r#"
C:\Users\User\project\src\main.rs
<<<<<<< SEARCH
old
=======
new
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(windows_path).unwrap();
        assert_eq!(blocks[0].file_path, r"C:\Users\User\project\src\main.rs");

        // Path with spaces
        let spaced_path = r#"
src/file with spaces.rs
<<<<<<< SEARCH
old
=======
new
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(spaced_path).unwrap();
        assert_eq!(blocks[0].file_path, "src/file with spaces.rs");
    }

    #[test]
    fn test_edge_cases() {
        // Multiple separators (should take first)
        let multiple_separators = r#"
src/test.rs
<<<<<<< SEARCH
old content
=======
intermediate
=======
new content
>>>>>>> REPLACE
"#;
        let blocks = parse_sr_blocks(multiple_separators).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].search_lines, "old content");
        assert!(blocks[0].replace_lines.contains("intermediate"));
        assert!(blocks[0].replace_lines.contains("======="));

        // Empty input
        let empty = "";
        let blocks = parse_sr_blocks(empty).unwrap();
        assert_eq!(blocks.len(), 0);

        // No S/R blocks
        let no_blocks = "Just some regular text without any markers";
        let blocks = parse_sr_blocks(no_blocks).unwrap();
        assert_eq!(blocks.len(), 0);
    }
}
