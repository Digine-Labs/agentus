/// A request to execute an LLM prompt.
pub struct ExecRequest {
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
}

/// The boundary between the VM and the outside world (LLM providers, tools).
pub trait HostInterface {
    /// Execute an LLM prompt and return the response text.
    fn exec(&self, request: ExecRequest) -> Result<String, String>;
}

/// Echo host: returns the user prompt as the response. For testing.
pub struct EchoHost;

impl HostInterface for EchoHost {
    fn exec(&self, request: ExecRequest) -> Result<String, String> {
        Ok(request.user_prompt)
    }
}

/// No-op host: errors if exec is called. Default when no host is configured.
pub struct NoHost;

impl HostInterface for NoHost {
    fn exec(&self, _request: ExecRequest) -> Result<String, String> {
        Err("no host configured: cannot execute LLM prompts".to_string())
    }
}
