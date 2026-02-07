use agentus_common::span::Span;

/// The root of an Agentus program.
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

/// A statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let name = expr` or `let name: type = expr`
    Let(LetStmt),
    /// `emit expr`
    Emit(EmitStmt),
    /// `return expr` or `return`
    Return(ReturnStmt),
    /// An expression used as a statement.
    ExprStmt(Expr),
    /// `if condition { ... } else { ... }`
    If(IfStmt),
    /// `while condition { ... }`
    While(WhileStmt),
    /// `for name in expr { ... }`
    For(ForStmt),
    /// `fn name(params) -> return_type { body }`
    FnDef(FnDef),
    /// Variable assignment: `name = expr`
    Assign(AssignStmt),
    /// Agent definition: `agent Name { ... }`
    AgentDef(AgentDef),
    /// Field assignment: `self.field = expr`
    FieldAssign(FieldAssignStmt),
    /// Tool definition: `tool name { ... }`
    ToolDef(ToolDef),
    /// Send message: `send target, message`
    Send(SendStmt),
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: String,
    pub type_ann: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EmitStmt {
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_body: Vec<Stmt>,
    pub else_body: Option<Vec<Stmt>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub variable: String,
    pub iterable: Expr,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AgentDef {
    pub name: String,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub memory_fields: Vec<MemoryField>,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MemoryField {
    pub name: String,
    pub type_ann: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldAssignStmt {
    pub object: Expr,
    pub field: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SendStmt {
    pub target: Expr,
    pub message: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: Option<String>,
    pub params: Vec<ToolParam>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub type_ann: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeExpr,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Str,
    Num,
    Bool,
    List(Box<TypeExpr>),
    Map(Box<TypeExpr>, Box<TypeExpr>),
    Optional(Box<TypeExpr>),
    AgentHandle,
}

/// A segment of a template/interpolated string.
#[derive(Debug, Clone)]
pub enum TemplateSegment {
    /// Literal text portion.
    Literal(String),
    /// An interpolated expression.
    Expr(Expr),
}

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// String literal
    StringLit(String, Span),
    /// Template/interpolated string: "hello {name}!"
    TemplateLit(Vec<TemplateSegment>, Span),
    /// Number literal
    NumberLit(f64, Span),
    /// Boolean literal
    BoolLit(bool, Span),
    /// None literal
    NoneLit(Span),
    /// Variable reference
    Ident(String, Span),
    /// Binary operation: left op right
    BinOp(Box<Expr>, BinOp, Box<Expr>, Span),
    /// Unary operation: op expr
    UnaryOp(UnaryOp, Box<Expr>, Span),
    /// Function call: name(args...)
    FnCall(String, Vec<Expr>, Span),
    /// Method call: obj.method(args...)
    MethodCall(Box<Expr>, String, Vec<Expr>, Span),
    /// Field access: expr.field
    FieldAccess(Box<Expr>, String, Span),
    /// Index access: expr[index]
    IndexAccess(Box<Expr>, Box<Expr>, Span),
    /// List literal: [a, b, c]
    ListLit(Vec<Expr>, Span),
    /// Map literal: { "key": value, ... }
    MapLit(Vec<(Expr, Expr)>, Span),
    /// Exec block: exec { prompt_expr }
    ExecBlock(Box<Expr>, Span),
    /// Recv expression: recv agent_handle
    Recv(Box<Expr>, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::StringLit(_, s) => *s,
            Expr::TemplateLit(_, s) => *s,
            Expr::NumberLit(_, s) => *s,
            Expr::BoolLit(_, s) => *s,
            Expr::NoneLit(s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::BinOp(_, _, _, s) => *s,
            Expr::UnaryOp(_, _, s) => *s,
            Expr::FnCall(_, _, s) => *s,
            Expr::MethodCall(_, _, _, s) => *s,
            Expr::FieldAccess(_, _, s) => *s,
            Expr::IndexAccess(_, _, s) => *s,
            Expr::ListLit(_, s) => *s,
            Expr::MapLit(_, s) => *s,
            Expr::ExecBlock(_, s) => *s,
            Expr::Recv(_, s) => *s,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Concat,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}
