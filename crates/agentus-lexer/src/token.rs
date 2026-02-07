use agentus_common::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// The raw source text of this token.
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, lexeme: String) -> Self {
        Self { kind, span, lexeme }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Literals
    StringLit,
    NumberLit,
    True,
    False,
    None,

    // Identifier
    Ident,

    // Keywords
    Agent,
    Tool,
    Pipeline,
    Stage,
    Fn,
    Let,
    Return,
    If,
    Else,
    For,
    In,
    While,
    Match,
    Try,
    Catch,
    Throw,
    Spawn,
    Send,
    Recv,
    Wait,
    Kill,
    Exec,
    Assert,
    Retry,
    Emit,
    Log,
    Use,
    Module,
    SelfKw,
    Parallel,
    Run,
    And,
    Or,
    Not,
    Memory,
    System,
    Prompt,
    Model,
    Tools,
    Description,
    Param,
    Required,
    Default,
    Returns,

    // Type keywords
    StrType,
    NumType,
    BoolType,
    ListType,
    MapType,
    AgentHandle,

    // Symbols
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    Dot,       // .
    Arrow,     // ->
    FatArrow,  // =>
    LeftArrow, // <-
    Question,  // ?
    DotDot,    // ..

    // Operators
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Percent,  // %
    PlusPlus, // ++

    // Comparison
    EqEq,   // ==
    BangEq, // !=
    Lt,     // <
    Lte,    // <=
    Gt,     // >
    Gte,    // >=

    // Assignment
    Assign, // =

    // Logical (keyword-based: and, or, not — see Keywords above)

    // String interpolation
    InterpStart, // { inside a string
    InterpEnd,   // } ending an interpolation

    // Special
    Newline,
    Eof,
    Error,
}

impl TokenKind {
    /// Try to match an identifier string to a keyword.
    pub fn keyword(ident: &str) -> Option<TokenKind> {
        match ident {
            "agent" => Some(TokenKind::Agent),
            "tool" => Some(TokenKind::Tool),
            "pipeline" => Some(TokenKind::Pipeline),
            "stage" => Some(TokenKind::Stage),
            "fn" => Some(TokenKind::Fn),
            "let" => Some(TokenKind::Let),
            "return" => Some(TokenKind::Return),
            "if" => Some(TokenKind::If),
            "else" => Some(TokenKind::Else),
            "for" => Some(TokenKind::For),
            "in" => Some(TokenKind::In),
            "while" => Some(TokenKind::While),
            "match" => Some(TokenKind::Match),
            "try" => Some(TokenKind::Try),
            "catch" => Some(TokenKind::Catch),
            "throw" => Some(TokenKind::Throw),
            "spawn" => Some(TokenKind::Spawn),
            "send" => Some(TokenKind::Send),
            "recv" => Some(TokenKind::Recv),
            "wait" => Some(TokenKind::Wait),
            "kill" => Some(TokenKind::Kill),
            "exec" => Some(TokenKind::Exec),
            "assert" => Some(TokenKind::Assert),
            "retry" => Some(TokenKind::Retry),
            "emit" => Some(TokenKind::Emit),
            "log" => Some(TokenKind::Log),
            "use" => Some(TokenKind::Use),
            "module" => Some(TokenKind::Module),
            "self" => Some(TokenKind::SelfKw),
            "parallel" => Some(TokenKind::Parallel),
            "run" => Some(TokenKind::Run),
            "and" => Some(TokenKind::And),
            "or" => Some(TokenKind::Or),
            "not" => Some(TokenKind::Not),
            "true" => Some(TokenKind::True),
            "false" => Some(TokenKind::False),
            "none" => Some(TokenKind::None),
            "memory" => Some(TokenKind::Memory),
            "system" => Some(TokenKind::System),
            "prompt" => Some(TokenKind::Prompt),
            "model" => Some(TokenKind::Model),
            "tools" => Some(TokenKind::Tools),
            "description" => Some(TokenKind::Description),
            "param" => Some(TokenKind::Param),
            "required" => Some(TokenKind::Required),
            "default" => Some(TokenKind::Default),
            "returns" => Some(TokenKind::Returns),
            "str" => Some(TokenKind::StrType),
            "num" => Some(TokenKind::NumType),
            "bool" => Some(TokenKind::BoolType),
            "list" => Some(TokenKind::ListType),
            "map" => Some(TokenKind::MapType),
            "agent_handle" => Some(TokenKind::AgentHandle),
            _ => Option::None,
        }
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
