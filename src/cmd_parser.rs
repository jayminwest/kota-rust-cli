use std::process::Command;
use regex::Regex;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct CommandBlock {
    pub command: String,
    pub description: Option<String>,
}

pub fn parse_command_blocks(response: &str) -> Result<Vec<CommandBlock>> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = response.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Look for command block markers
        if lines[i].trim().starts_with("```bash") || lines[i].trim().starts_with("```sh") || lines[i].trim() == "```command" {
            i += 1; // Skip the opening marker
            
            let mut command_lines = Vec::new();
            let mut found_end = false;
            
            // Collect command lines until we find the closing ```
            while i < lines.len() {
                let line = lines[i].trim();
                if line == "```" {
                    found_end = true;
                    i += 1;
                    break;
                }
                command_lines.push(line);
                i += 1;
            }
            
            if !found_end {
                return Err(anyhow::anyhow!("Malformed command block: missing closing ```"));
            }
            
            if !command_lines.is_empty() {
                let command = command_lines.join(" && ");
                blocks.push(CommandBlock {
                    command,
                    description: None,
                });
            }
        } else {
            i += 1;
        }
    }
    
    Ok(blocks)
}

pub fn contains_command_blocks(response: &str) -> bool {
    let command_pattern = Regex::new(r"```(?:bash|sh|command)").unwrap();
    command_pattern.is_match(response)
}

pub async fn execute_command(cmd: &str) -> Result<(String, String, bool)> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute command '{}': {}", cmd, e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();
    
    Ok((stdout, stderr, success))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_blocks() {
        let input = r#"Here's what we need to do:

```bash
ls -la
cd src
grep -r "main" .
```

And then run:

```sh
cargo build
```

That should work!"#;

        let blocks = parse_command_blocks(input).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].command, "ls -la && cd src && grep -r \"main\" .");
        assert_eq!(blocks[1].command, "cargo build");
    }

    #[test]
    fn test_contains_command_blocks() {
        assert!(contains_command_blocks("some text ```bash\nls\n``` more text"));
        assert!(contains_command_blocks("```sh\necho hello\n```"));
        assert!(!contains_command_blocks("regular text without command blocks"));
    }
}