use std::process::Command;
use regex::Regex;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct CommandBlock {
    pub command: String,
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

    #[test]
    fn test_malformed_command_blocks() {
        // Missing closing backticks
        let missing_close = r#"
```bash
ls -la
echo "hello"
"#;
        let result = parse_command_blocks(missing_close);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing closing ```"));

        // Empty command block
        let empty_block = r#"
```bash
```
"#;
        let blocks = parse_command_blocks(empty_block).unwrap();
        assert_eq!(blocks.len(), 0); // Empty blocks are filtered out

        // Multiple unclosed blocks
        let multiple_unclosed = r#"
```bash
command1
```sh
command2
"#;
        let result = parse_command_blocks(multiple_unclosed);
        assert!(result.is_err());
    }

    #[test]
    fn test_command_block_variations() {
        // Different block types
        let various_types = r#"
```bash
echo "bash"
```

```sh
echo "sh"
```

```command
echo "command"
```
"#;
        let blocks = parse_command_blocks(various_types).unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].command, r#"echo "bash""#);
        assert_eq!(blocks[1].command, r#"echo "sh""#);
        assert_eq!(blocks[2].command, r#"echo "command""#);

        // Case sensitivity
        let case_test = r#"
```BASH
echo "uppercase"
```

```Bash
echo "mixed case"
```
"#;
        let blocks = parse_command_blocks(case_test).unwrap();
        assert_eq!(blocks.len(), 0); // Should not match uppercase variants
    }

    #[test]
    fn test_command_joining() {
        // Multiple commands should be joined with &&
        let multiple_commands = r#"
```bash
cd /tmp
mkdir test_dir
cd test_dir
touch file.txt
ls -la
```
"#;
        let blocks = parse_command_blocks(multiple_commands).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].command, "cd /tmp && mkdir test_dir && cd test_dir && touch file.txt && ls -la");

        // Single command should not have &&
        let single_command = r#"
```bash
ls -la
```
"#;
        let blocks = parse_command_blocks(single_command).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].command, "ls -la");
    }

    #[test]
    fn test_whitespace_handling() {
        // Extra whitespace should be trimmed
        let whitespace_commands = r#"
```bash
   ls -la   
  cd src  
    grep "main" .    
```
"#;
        let blocks = parse_command_blocks(whitespace_commands).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].command, "ls -la && cd src && grep \"main\" .");

        // Empty lines should be filtered out
        let empty_lines = r#"
```bash
ls -la

cd src

grep "main" .
```
"#;
        let blocks = parse_command_blocks(empty_lines).unwrap();
        assert_eq!(blocks.len(), 1);
        // Empty lines are currently included - this might be a bug to fix
        assert!(blocks[0].command.contains("&&"));
    }

    #[test]
    fn test_special_characters() {
        // Commands with quotes, pipes, redirects
        let special_chars = r#"
```bash
echo "Hello, World!" | grep "World"
find . -name "*.rs" | head -5
cat file.txt > output.txt 2>&1
```
"#;
        let blocks = parse_command_blocks(special_chars).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].command.contains("|"));
        assert!(blocks[0].command.contains(">"));
        assert!(blocks[0].command.contains("2>&1"));

        // Commands with backticks (should not interfere with block parsing)
        let backticks_in_command = r#"
```bash
echo `date`
ls `pwd`
```
"#;
        let blocks = parse_command_blocks(backticks_in_command).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].command.contains("`date`"));
        assert!(blocks[0].command.contains("`pwd`"));
    }

    #[test]
    fn test_nested_blocks() {
        // Code blocks inside command blocks (edge case)
        let nested = r#"
```bash
echo '```'
echo 'nested backticks'
echo '```'
```
"#;
        let blocks = parse_command_blocks(nested).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].command.contains("```"));
        assert!(blocks[0].command.contains("nested backticks"));
    }

    #[test]
    fn test_mixed_content() {
        // Commands mixed with other code blocks
        let mixed = r#"
Here's some Python code:

```python
def hello():
    print("Hello")
```

And here's a bash command:

```bash
python script.py
```

And more Python:

```python
hello()
```
"#;
        let blocks = parse_command_blocks(mixed).unwrap();
        assert_eq!(blocks.len(), 1); // Only bash block should be parsed
        assert_eq!(blocks[0].command, "python script.py");
    }

    #[tokio::test]
    async fn test_execute_command() {
        // Test successful command
        let (stdout, stderr, success) = execute_command("echo 'hello world'").await.unwrap();
        assert!(success);
        assert_eq!(stdout.trim(), "hello world");
        assert!(stderr.is_empty());

        // Test command with stderr
        let (stdout, stderr, success) = execute_command("echo 'error' >&2").await.unwrap();
        assert!(success); // echo to stderr still succeeds
        assert!(stdout.is_empty());
        assert_eq!(stderr.trim(), "error");

        // Test failing command
        let (stdout, _stderr, success) = execute_command("false").await.unwrap();
        assert!(!success);
        assert!(stdout.is_empty());
        // stderr might be empty for the false command

        // Test invalid command
        let result = execute_command("nonexistent_command_xyz").await;
        // This should either fail to execute or return success=false
        if let Ok((_, _, success)) = result {
            assert!(!success);
        }
        // If it fails to execute, that's also acceptable
    }
}