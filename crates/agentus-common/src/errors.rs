use crate::span::Span;

/// Unified error type for the Agentus compiler and runtime.
#[derive(Debug, thiserror::Error)]
pub enum AgentusError {
    #[error("Lexer error at {span:?}: {message}")]
    LexerError { message: String, span: Span },

    #[error("Parser error at {span:?}: {message}")]
    ParserError { message: String, span: Span },

    #[error("Semantic error at {span:?}: {message}")]
    SemanticError { message: String, span: Span },

    #[error("Codegen error: {message}")]
    CodegenError { message: String },

    #[error("Runtime error: {message}")]
    RuntimeError { message: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
