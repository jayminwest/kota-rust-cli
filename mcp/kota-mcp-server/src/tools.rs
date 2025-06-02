use serde_json::{json, Value};

pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "send_to_mac_pro",
            "description": "Send data, commands, or insights to the Mac Pro system through the bridge",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Category of data (knowledge, context, insight, command, analysis)",
                        "enum": ["knowledge", "context", "insight", "command", "analysis", "file_update", "project_status"]
                    },
                    "content": {
                        "type": "string", 
                        "description": "The content to send"
                    },
                    "metadata": {
                        "type": "object",
                        "description": "Optional metadata about the content",
                        "properties": {
                            "priority": {"type": "string", "enum": ["low", "medium", "high", "urgent"]},
                            "source": {"type": "string"},
                            "tags": {"type": "array", "items": {"type": "string"}},
                            "related_files": {"type": "array", "items": {"type": "string"}}
                        }
                    }
                },
                "required": ["category", "content"]
            }
        }),
        json!({
            "name": "query_mac_pro_data",
            "description": "Query data from Mac Pro MCP servers (calendar, finance, email, etc.)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "server_name": {
                        "type": "string",
                        "description": "Name of the MCP server to query",
                        "enum": ["google-calendar", "plaid-finance", "gmail", "notion", "slack", "github"]
                    },
                    "tool_name": {
                        "type": "string",
                        "description": "Name of the tool/function to call"
                    },
                    "arguments": {
                        "type": "object",
                        "description": "Arguments to pass to the tool"
                    }
                },
                "required": ["server_name", "tool_name"]
            }
        }),
        json!({
            "name": "get_mac_pro_status",
            "description": "Get comprehensive system status and health information from Mac Pro",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_bridge_logs",
            "description": "Get communication logs from the bridge server for debugging or analysis",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of log entries to return",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 1000
                    },
                    "level": {
                        "type": "string",
                        "description": "Log level filter",
                        "enum": ["error", "warn", "info", "debug"],
                        "default": "info"
                    }
                }
            }
        }),
        json!({
            "name": "analyze_kota_context",
            "description": "Analyze current KOTA CLI context and generate insights about the development session",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "analysis_type": {
                        "type": "string",
                        "enum": ["files", "commands", "patterns", "summary", "productivity", "recommendations"],
                        "description": "Type of analysis to perform on the current context"
                    },
                    "focus_area": {
                        "type": "string",
                        "description": "Specific area to focus the analysis on (optional)",
                        "enum": ["performance", "security", "architecture", "testing", "documentation"]
                    }
                },
                "required": ["analysis_type"]
            }
        }),
        json!({
            "name": "send_proactive_insight",
            "description": "Send a proactive insight or recommendation to Mac Pro for context awareness",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "insight": {
                        "type": "string",
                        "description": "The insight or recommendation to send"
                    },
                    "confidence": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Confidence level of the insight (0-1)"
                    },
                    "category": {
                        "type": "string",
                        "description": "Category of insight",
                        "enum": ["productivity", "schedule", "code_quality", "performance", "security", "optimization", "learning"]
                    },
                    "urgency": {
                        "type": "string",
                        "description": "Urgency level of the insight",
                        "enum": ["low", "medium", "high", "immediate"],
                        "default": "medium"
                    },
                    "actionable": {
                        "type": "boolean",
                        "description": "Whether this insight contains actionable recommendations",
                        "default": true
                    }
                },
                "required": ["insight", "confidence"]
            }
        }),
        json!({
            "name": "sync_project_status",
            "description": "Synchronize current project status and development progress with Mac Pro",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_name": {
                        "type": "string",
                        "description": "Name of the current project"
                    },
                    "status": {
                        "type": "string",
                        "description": "Current project status",
                        "enum": ["starting", "in_progress", "testing", "debugging", "documenting", "completed", "blocked"]
                    },
                    "progress_summary": {
                        "type": "string",
                        "description": "Brief summary of recent progress"
                    },
                    "next_steps": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Planned next steps or tasks"
                    },
                    "blockers": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Current blockers or issues"
                    }
                },
                "required": ["project_name", "status"]
            }
        }),
        json!({
            "name": "request_mac_pro_assistance",
            "description": "Request specific assistance or data from Mac Pro systems",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "assistance_type": {
                        "type": "string",
                        "description": "Type of assistance needed",
                        "enum": ["calendar_check", "financial_data", "research", "scheduling", "reminders", "document_search"]
                    },
                    "request_details": {
                        "type": "string",
                        "description": "Detailed description of what assistance is needed"
                    },
                    "priority": {
                        "type": "string",
                        "description": "Priority level of the request",
                        "enum": ["low", "medium", "high", "urgent"],
                        "default": "medium"
                    },
                    "context": {
                        "type": "object",
                        "description": "Additional context for the request"
                    }
                },
                "required": ["assistance_type", "request_details"]
            }
        })
    ]
}