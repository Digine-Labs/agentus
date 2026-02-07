use std::collections::HashMap;
use agentus_parser::ast::*;

/// Minimal semantic analysis: name resolution and scope checking.
///
/// Ensures all variables are defined before use and tracks scopes.
pub struct Resolver {
    /// Stack of scopes. Each scope maps variable names to a "defined" flag.
    scopes: Vec<HashMap<String, bool>>,
    errors: Vec<String>,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // global scope
            errors: Vec::new(),
        }
    }

    /// Resolve the given program, returning any errors found.
    pub fn resolve(mut self, program: &Program) -> Result<(), Vec<String>> {
        for stmt in &program.statements {
            self.resolve_stmt(stmt);
        }
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), true);
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }
        false
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(l) => {
                self.resolve_expr(&l.value);
                self.define(&l.name);
            }
            Stmt::Emit(e) => {
                self.resolve_expr(&e.value);
            }
            Stmt::Return(r) => {
                if let Some(v) = &r.value {
                    self.resolve_expr(v);
                }
            }
            Stmt::ExprStmt(e) => {
                self.resolve_expr(e);
            }
            Stmt::Assign(a) => {
                if !self.is_defined(&a.name) {
                    self.errors.push(format!(
                        "undefined variable '{}' at {:?}",
                        a.name, a.span
                    ));
                }
                self.resolve_expr(&a.value);
            }
            Stmt::If(i) => {
                self.resolve_expr(&i.condition);
                self.push_scope();
                for s in &i.then_body {
                    self.resolve_stmt(s);
                }
                self.pop_scope();
                if let Some(else_body) = &i.else_body {
                    self.push_scope();
                    for s in else_body {
                        self.resolve_stmt(s);
                    }
                    self.pop_scope();
                }
            }
            Stmt::While(w) => {
                self.resolve_expr(&w.condition);
                self.push_scope();
                for s in &w.body {
                    self.resolve_stmt(s);
                }
                self.pop_scope();
            }
            Stmt::For(f) => {
                self.resolve_expr(&f.iterable);
                self.push_scope();
                self.define(&f.variable);
                for s in &f.body {
                    self.resolve_stmt(s);
                }
                self.pop_scope();
            }
            Stmt::FnDef(f) => {
                self.define(&f.name);
                self.push_scope();
                for p in &f.params {
                    self.define(&p.name);
                }
                for s in &f.body {
                    self.resolve_stmt(s);
                }
                self.pop_scope();
            }
            Stmt::AgentDef(a) => {
                self.define(&a.name);
                self.push_scope();
                self.define("self");
                for field in &a.memory_fields {
                    if let Some(default) = &field.default {
                        self.resolve_expr(default);
                    }
                }
                for method in &a.methods {
                    self.define(&method.name);
                    self.push_scope();
                    for p in &method.params {
                        self.define(&p.name);
                    }
                    for s in &method.body {
                        self.resolve_stmt(s);
                    }
                    self.pop_scope();
                }
                self.pop_scope();
            }
            Stmt::FieldAssign(fa) => {
                self.resolve_expr(&fa.object);
                self.resolve_expr(&fa.value);
            }
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::StringLit(_, _)
            | Expr::NumberLit(_, _)
            | Expr::BoolLit(_, _)
            | Expr::NoneLit(_) => {}
            Expr::TemplateLit(segments, _) => {
                for seg in segments {
                    if let agentus_parser::ast::TemplateSegment::Expr(e) = seg {
                        self.resolve_expr(e);
                    }
                }
            }
            Expr::Ident(name, span) => {
                if !self.is_defined(name) {
                    self.errors
                        .push(format!("undefined variable '{}' at {:?}", name, span));
                }
            }
            Expr::BinOp(left, _, right, _) => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::UnaryOp(_, expr, _) => {
                self.resolve_expr(expr);
            }
            Expr::FnCall(_, args, _) => {
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::MethodCall(obj, _, args, _) => {
                self.resolve_expr(obj);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::FieldAccess(obj, _, _) => {
                self.resolve_expr(obj);
            }
            Expr::IndexAccess(obj, index, _) => {
                self.resolve_expr(obj);
                self.resolve_expr(index);
            }
            Expr::ListLit(elems, _) => {
                for elem in elems {
                    self.resolve_expr(elem);
                }
            }
            Expr::MapLit(pairs, _) => {
                for (k, v) in pairs {
                    self.resolve_expr(k);
                    self.resolve_expr(v);
                }
            }
            Expr::ExecBlock(prompt, _) => {
                self.resolve_expr(prompt);
            }
        }
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: resolve a program.
pub fn resolve(program: &Program) -> Result<(), Vec<String>> {
    Resolver::new().resolve(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentus_parser::parser::parse;

    #[test]
    fn test_valid_program() {
        let program = parse("let x = 42\nemit x").unwrap();
        assert!(resolve(&program).is_ok());
    }

    #[test]
    fn test_undefined_variable() {
        let program = parse("emit x").unwrap();
        let result = resolve(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("undefined variable 'x'"));
    }

    #[test]
    fn test_scope_in_if() {
        // Variable defined in if body shouldn't leak
        let src = "let x = 1\nif x > 0 {\n    let y = 2\n}";
        let program = parse(src).unwrap();
        assert!(resolve(&program).is_ok());
    }

    #[test]
    fn test_function_params_in_scope() {
        let src = "fn add(a: num, b: num) -> num {\n    return a + b\n}";
        let program = parse(src).unwrap();
        assert!(resolve(&program).is_ok());
    }

    #[test]
    fn test_fn_name_defined() {
        let src = "fn foo() -> num {\n    return 1\n}\nlet x = foo()";
        let program = parse(src).unwrap();
        assert!(resolve(&program).is_ok());
    }
}
