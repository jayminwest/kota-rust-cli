
#[derive(Clone)]
pub enum InputMode {
    Normal,
    Insert,
    Command,
    FileBrowser,
}

#[derive(Clone)]
pub enum FocusedPane {
    Chat,
    Terminal,
    Context,
    FileBrowser,
}

#[derive(Clone)]
pub enum AppMessage {
    LlmResponse(String, String), // (original_prompt, response)
    TerminalOutput(String),
    ProcessingComplete,
}

#[derive(Clone)]
pub enum MessageContent {
    Text(String),
    CollapsedPaste { 
        summary: String,  // e.g., "[Pasted 150 lines]"
        full_content: String,  // The actual pasted content
    },
}

#[derive(Clone, Debug)]
pub enum CommandStatus {
    Pending,
    Running,
    Success,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct CommandSuggestion {
    pub command: String,
    pub description: Option<String>,
    pub status: CommandStatus,
    pub output: Option<String>,
}