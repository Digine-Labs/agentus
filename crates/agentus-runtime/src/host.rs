/// A request to execute an LLM prompt.
pub struct ExecRequest {
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
}

/// A request to call a tool.
pub struct ToolCallRequest {
    pub tool_name: String,
    pub args: Vec<(String, String)>,
}

/// The boundary between the VM and the outside world (LLM providers, tools).
pub trait HostInterface {
    /// Execute an LLM prompt and return the response text.
    fn exec(&self, request: ExecRequest) -> Result<String, String>;

    /// Call a tool with named arguments and return the result text.
    fn tool_call(&self, request: ToolCallRequest) -> Result<String, String>;
}

/// Echo host: returns the user prompt as the response. For testing.
pub struct EchoHost;

impl HostInterface for EchoHost {
    fn exec(&self, request: ExecRequest) -> Result<String, String> {
        Ok(request.user_prompt)
    }

    fn tool_call(&self, request: ToolCallRequest) -> Result<String, String> {
        // Return a formatted string showing the tool call for testing
        let args_str: Vec<String> = request
            .args
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        Ok(format!("{}({})", request.tool_name, args_str.join(", ")))
    }
}

/// No-op host: errors if exec or tool_call is called. Default when no host is configured.
pub struct NoHost;

impl HostInterface for NoHost {
    fn exec(&self, _request: ExecRequest) -> Result<String, String> {
        Err("no host configured: cannot execute LLM prompts".to_string())
    }

    fn tool_call(&self, _request: ToolCallRequest) -> Result<String, String> {
        Err("no host configured: cannot call tools".to_string())
    }
}
