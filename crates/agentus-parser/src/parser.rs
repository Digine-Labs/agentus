use agentus_common::span::Span;
use agentus_lexer::token::{Token, TokenKind};
use crate::ast::*;

/// The Agentus parser. Recursive descent with Pratt parsing for expressions.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    /// Parse the token stream into a Program.
    pub fn parse(mut self) -> Result<Program, Vec<String>> {
        let start_span = self.current_span();
        let mut statements = Vec::new();

        self.skip_newlines();
        while !self.is_at_end() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
            self.skip_newlines();
        }

        if self.errors.is_empty() {
            let end_span = if statements.is_empty() {
                start_span
            } else {
                self.tokens.last().map(|t| t.span).unwrap_or(start_span)
            };
            Ok(Program {
                statements,
                span: start_span.merge(end_span),
            })
        } else {
            Err(self.errors)
        }
    }

    // =====================================================================
    // Statement parsing
    // =====================================================================

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match self.current_kind() {
            TokenKind::Let => self.parse_let(),
            TokenKind::Emit => self.parse_emit(),
            TokenKind::Return => self.parse_return(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Fn => self.parse_fn_def(),
            TokenKind::Agent => self.parse_agent_def(),
            TokenKind::Tool => self.parse_tool_def(),
            TokenKind::Send => self.parse_send(),
            TokenKind::Try => self.parse_try_catch(),
            TokenKind::Throw => self.parse_throw(),
            TokenKind::Assert => self.parse_assert(),
            _ => {
                // Try to parse as expression statement or assignment
                let expr = self.parse_expression(0)?;
                // Check for assignment
                if let Expr::Ident(ref name, _) = expr {
                    if self.current_kind() == TokenKind::Assign {
                        let start_span = expr.span();
                        self.advance(); // consume =
                        let value = self.parse_expression(0)?;
                        let span = start_span.merge(value.span());
                        self.expect_statement_end()?;
                        return Ok(Stmt::Assign(AssignStmt {
                            name: name.clone(),
                            value,
                            span,
                        }));
                    }
                }
                // Check for field assignment: expr.field = value
                if let Expr::FieldAccess(ref obj, ref field, _) = expr {
                    if self.current_kind() == TokenKind::Assign {
                        let start_span = expr.span();
                        self.advance(); // consume =
                        let value = self.parse_expression(0)?;
                        let span = start_span.merge(value.span());
                        self.expect_statement_end()?;
                        return Ok(Stmt::FieldAssign(FieldAssignStmt {
                            object: *obj.clone(),
                            field: field.clone(),
                            value,
                            span,
                        }));
                    }
                }
                // Check for index assignment: expr[key] = value
                if let Expr::IndexAccess(ref obj, ref index, _) = expr {
                    if self.current_kind() == TokenKind::Assign {
                        let start_span = expr.span();
                        self.advance(); // consume =
                        let value = self.parse_expression(0)?;
                        let span = start_span.merge(value.span());
                        self.expect_statement_end()?;
                        return Ok(Stmt::IndexAssign(IndexAssignStmt {
                            object: *obj.clone(),
                            index: *index.clone(),
                            value,
                            span,
                        }));
                    }
                }
                self.expect_statement_end()?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Let)?;

        let name = self.expect_ident()?;

        // Optional type annotation
        let type_ann = if self.current_kind() == TokenKind::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            Option::None
        };

        self.expect(TokenKind::Assign)?;
        let value = self.parse_expression(0)?;
        let span = start.merge(value.span());
        self.expect_statement_end()?;

        Ok(Stmt::Let(LetStmt {
            name,
            type_ann,
            value,
            span,
        }))
    }

    fn parse_emit(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Emit)?;
        let value = self.parse_expression(0)?;
        let span = start.merge(value.span());
        self.expect_statement_end()?;
        Ok(Stmt::Emit(EmitStmt { value, span }))
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Return)?;

        let value = if self.is_at_statement_end() {
            Option::None
        } else {
            Some(self.parse_expression(0)?)
        };

        let span = match &value {
            Some(v) => start.merge(v.span()),
            Option::None => start,
        };
        self.expect_statement_end()?;
        Ok(Stmt::Return(ReturnStmt { value, span }))
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::If)?;

        let condition = self.parse_expression(0)?;
        self.expect(TokenKind::LBrace)?;
        let then_body = self.parse_block()?;
        self.expect(TokenKind::RBrace)?;

        let else_body = if self.current_kind() == TokenKind::Else {
            self.advance();
            if self.current_kind() == TokenKind::If {
                // else if -> wrap in a single-element vec
                let if_stmt = self.parse_if()?;
                Some(vec![if_stmt])
            } else {
                self.expect(TokenKind::LBrace)?;
                let body = self.parse_block()?;
                self.expect(TokenKind::RBrace)?;
                Some(body)
            }
        } else {
            Option::None
        };

        let span = start.merge(self.prev_span());
        self.expect_statement_end()?;
        Ok(Stmt::If(IfStmt {
            condition,
            then_body,
            else_body,
            span,
        }))
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::While)?;
        let condition = self.parse_expression(0)?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block()?;
        self.expect(TokenKind::RBrace)?;
        let span = start.merge(self.prev_span());
        self.expect_statement_end()?;
        Ok(Stmt::While(WhileStmt {
            condition,
            body,
            span,
        }))
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::For)?;
        let variable = self.expect_ident()?;
        self.expect(TokenKind::In)?;
        let iterable = self.parse_expression(0)?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block()?;
        self.expect(TokenKind::RBrace)?;
        let span = start.merge(self.prev_span());
        self.expect_statement_end()?;
        Ok(Stmt::For(ForStmt {
            variable,
            iterable,
            body,
            span,
        }))
    }

    fn parse_fn_def(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;

        let return_type = if self.current_kind() == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            Option::None
        };

        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block()?;
        self.expect(TokenKind::RBrace)?;
        let span = start.merge(self.prev_span());
        self.expect_statement_end()?;

        Ok(Stmt::FnDef(FnDef {
            name,
            params,
            return_type,
            body,
            span,
        }))
    }

    fn parse_agent_def(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Agent)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut model = None;
        let mut system_prompt = None;
        let mut memory_fields = Vec::new();
        let mut methods = Vec::new();

        while self.current_kind() != TokenKind::RBrace && !self.is_at_end() {
            match self.current_kind() {
                TokenKind::Model => {
                    self.advance(); // consume 'model'
                    self.expect(TokenKind::Assign)?;
                    if self.current_kind() == TokenKind::StringLit {
                        let token = self.advance_and_get();
                        model = Some(token.lexeme);
                    } else {
                        return Err(format!(
                            "expected string for model, found {:?} at {:?}",
                            self.current_kind(),
                            self.current_span()
                        ));
                    }
                    self.skip_newlines();
                }
                TokenKind::System => {
                    self.advance(); // consume 'system'
                    self.expect(TokenKind::Prompt)?;
                    self.expect(TokenKind::LBrace)?;
                    self.skip_newlines();
                    if self.current_kind() == TokenKind::StringLit {
                        let token = self.advance_and_get();
                        system_prompt = Some(token.lexeme);
                    } else {
                        return Err(format!(
                            "expected string for system prompt, found {:?} at {:?}",
                            self.current_kind(),
                            self.current_span()
                        ));
                    }
                    self.skip_newlines();
                    self.expect(TokenKind::RBrace)?;
                    self.skip_newlines();
                }
                TokenKind::Memory => {
                    self.advance(); // consume 'memory'
                    self.expect(TokenKind::LBrace)?;
                    self.skip_newlines();
                    while self.current_kind() != TokenKind::RBrace && !self.is_at_end() {
                        let field_start = self.current_span();
                        let field_name = self.expect_ident()?;
                        self.expect(TokenKind::Colon)?;
                        let type_ann = self.parse_type()?;
                        let default = if self.current_kind() == TokenKind::Assign {
                            self.advance();
                            Some(self.parse_expression(0)?)
                        } else {
                            Option::None
                        };
                        let field_span = field_start.merge(self.prev_span());
                        memory_fields.push(MemoryField {
                            name: field_name,
                            type_ann,
                            default,
                            span: field_span,
                        });
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace)?;
                    self.skip_newlines();
                }
                TokenKind::Fn => {
                    self.advance(); // consume 'fn'
                    let fn_start = self.prev_span();
                    let fn_name = self.expect_ident()?;
                    self.expect(TokenKind::LParen)?;
                    let params = self.parse_params()?;
                    self.expect(TokenKind::RParen)?;
                    let return_type = if self.current_kind() == TokenKind::Arrow {
                        self.advance();
                        Some(self.parse_type()?)
                    } else {
                        Option::None
                    };
                    self.expect(TokenKind::LBrace)?;
                    let body = self.parse_block()?;
                    self.expect(TokenKind::RBrace)?;
                    let fn_span = fn_start.merge(self.prev_span());
                    methods.push(FnDef {
                        name: fn_name,
                        params,
                        return_type,
                        body,
                        span: fn_span,
                    });
                    self.skip_newlines();
                }
                _ => {
                    return Err(format!(
                        "unexpected token {:?} in agent definition at {:?}",
                        self.current_kind(),
                        self.current_span()
                    ));
                }
            }
        }

        self.expect(TokenKind::RBrace)?;
        let span = start.merge(self.prev_span());
        self.expect_statement_end()?;

        Ok(Stmt::AgentDef(AgentDef {
            name,
            model,
            system_prompt,
            memory_fields,
            methods,
            span,
        }))
    }

    fn parse_tool_def(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Tool)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut description = None;
        let mut params = Vec::new();
        let mut return_type = None;

        while self.current_kind() != TokenKind::RBrace && !self.is_at_end() {
            match self.current_kind() {
                TokenKind::Description => {
                    self.advance(); // consume 'description'
                    self.expect(TokenKind::LBrace)?;
                    self.skip_newlines();
                    if self.current_kind() == TokenKind::StringLit {
                        let token = self.advance_and_get();
                        description = Some(token.lexeme);
                    } else {
                        return Err(format!(
                            "expected string for tool description, found {:?} at {:?}",
                            self.current_kind(),
                            self.current_span()
                        ));
                    }
                    self.skip_newlines();
                    self.expect(TokenKind::RBrace)?;
                    self.skip_newlines();
                }
                TokenKind::Param => {
                    let param_start = self.current_span();
                    self.advance(); // consume 'param'
                    let param_name = self.expect_ident()?;
                    self.expect(TokenKind::Colon)?;
                    let type_ann = self.parse_type()?;
                    let default = if self.current_kind() == TokenKind::Assign {
                        self.advance();
                        Some(self.parse_expression(0)?)
                    } else {
                        Option::None
                    };
                    let param_span = param_start.merge(self.prev_span());
                    params.push(ToolParam {
                        name: param_name,
                        type_ann,
                        default,
                        span: param_span,
                    });
                    self.skip_newlines();
                }
                TokenKind::Returns => {
                    self.advance(); // consume 'returns'
                    return_type = Some(self.parse_type()?);
                    self.skip_newlines();
                }
                _ => {
                    return Err(format!(
                        "unexpected token {:?} in tool definition at {:?}",
                        self.current_kind(),
                        self.current_span()
                    ));
                }
            }
        }

        self.expect(TokenKind::RBrace)?;
        let span = start.merge(self.prev_span());
        self.expect_statement_end()?;

        Ok(Stmt::ToolDef(ToolDef {
            name,
            description,
            params,
            return_type,
            span,
        }))
    }

    fn parse_send(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Send)?;
        let target = self.parse_expression(0)?;
        self.expect(TokenKind::Comma)?;
        let message = self.parse_expression(0)?;
        let span = start.merge(message.span());
        self.expect_statement_end()?;
        Ok(Stmt::Send(SendStmt {
            target,
            message,
            span,
        }))
    }

    fn parse_try_catch(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Try)?;
        self.expect(TokenKind::LBrace)?;
        let try_body = self.parse_block()?;
        self.expect(TokenKind::RBrace)?;
        self.skip_newlines();
        self.expect(TokenKind::Catch)?;
        let catch_var = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let catch_body = self.parse_block()?;
        self.expect(TokenKind::RBrace)?;
        let span = start.merge(self.prev_span());
        self.expect_statement_end()?;
        Ok(Stmt::TryCatch(TryCatchStmt {
            try_body,
            catch_var,
            catch_body,
            span,
        }))
    }

    fn parse_throw(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Throw)?;
        let value = self.parse_expression(0)?;
        let span = start.merge(value.span());
        self.expect_statement_end()?;
        Ok(Stmt::Throw(ThrowStmt { value, span }))
    }

    fn parse_assert(&mut self) -> Result<Stmt, String> {
        let start = self.current_span();
        self.expect(TokenKind::Assert)?;
        let condition = self.parse_expression(0)?;
        let message = if self.current_kind() == TokenKind::Comma {
            self.advance(); // consume comma
            Some(self.parse_expression(0)?)
        } else {
            Option::None
        };
        let span = start.merge(self.prev_span());
        self.expect_statement_end()?;
        Ok(Stmt::Assert(AssertStmt {
            condition,
            message,
            span,
        }))
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();
        if self.current_kind() == TokenKind::RParen {
            return Ok(params);
        }

        loop {
            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let type_ann = self.parse_type()?;
            params.push(Param { name, type_ann });

            if self.current_kind() != TokenKind::Comma {
                break;
            }
            self.advance(); // consume comma
        }

        Ok(params)
    }

    fn parse_type(&mut self) -> Result<TypeExpr, String> {
        let base = match self.current_kind() {
            TokenKind::StrType => {
                self.advance();
                TypeExpr::Str
            }
            TokenKind::NumType => {
                self.advance();
                TypeExpr::Num
            }
            TokenKind::BoolType => {
                self.advance();
                TypeExpr::Bool
            }
            TokenKind::AgentHandle => {
                self.advance();
                TypeExpr::AgentHandle
            }
            TokenKind::ListType => {
                self.advance();
                self.expect(TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(TokenKind::RBracket)?;
                TypeExpr::List(Box::new(inner))
            }
            TokenKind::MapType => {
                self.advance();
                self.expect(TokenKind::LBracket)?;
                let key = self.parse_type()?;
                self.expect(TokenKind::Comma)?;
                let val = self.parse_type()?;
                self.expect(TokenKind::RBracket)?;
                TypeExpr::Map(Box::new(key), Box::new(val))
            }
            _ => {
                return Err(format!(
                    "expected type, found {:?} at {:?}",
                    self.current_kind(),
                    self.current_span()
                ));
            }
        };

        // Check for optional `?`
        if self.current_kind() == TokenKind::Question {
            self.advance();
            Ok(TypeExpr::Optional(Box::new(base)))
        } else {
            Ok(base)
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while self.current_kind() != TokenKind::RBrace && !self.is_at_end() {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    // =====================================================================
    // Expression parsing (Pratt / precedence climbing)
    // =====================================================================

    fn parse_expression(&mut self, min_prec: u8) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        while let Some((prec, assoc)) = self.current_binop_precedence() {
            if prec < min_prec {
                break;
            }
            let op = self.parse_binop()?;
            let next_prec = match assoc {
                Assoc::Left => prec + 1,
                Assoc::Right => prec,
            };
            let right = self.parse_expression(next_prec)?;
            let span = left.span().merge(right.span());
            left = Expr::BinOp(Box::new(left), op, Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.current_kind() {
            TokenKind::Minus => {
                let start = self.current_span();
                self.advance();
                let expr = self.parse_unary()?;
                let span = start.merge(expr.span());
                Ok(Expr::UnaryOp(UnaryOp::Neg, Box::new(expr), span))
            }
            TokenKind::Not => {
                let start = self.current_span();
                self.advance();
                let expr = self.parse_unary()?;
                let span = start.merge(expr.span());
                Ok(Expr::UnaryOp(UnaryOp::Not, Box::new(expr), span))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current_kind() {
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    // Check for method call: expr.field(args)
                    if self.current_kind() == TokenKind::LParen {
                        self.advance();
                        let args = self.parse_call_args()?;
                        self.expect(TokenKind::RParen)?;
                        let span = expr.span().merge(self.prev_span());
                        expr = Expr::MethodCall(Box::new(expr), field, args, span);
                    } else {
                        let span = expr.span().merge(self.prev_span());
                        expr = Expr::FieldAccess(Box::new(expr), field, span);
                    }
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expression(0)?;
                    self.expect(TokenKind::RBracket)?;
                    let span = expr.span().merge(self.prev_span());
                    expr = Expr::IndexAccess(Box::new(expr), Box::new(index), span);
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.current_kind() {
            TokenKind::StringLit => {
                let token = self.advance_and_get();
                let start_span = token.span;

                // Check if this is the beginning of an interpolated string
                if self.current_kind() == TokenKind::InterpStart {
                    // Build a template literal
                    let mut segments = Vec::new();
                    if !token.lexeme.is_empty() {
                        segments.push(TemplateSegment::Literal(token.lexeme));
                    }

                    while self.current_kind() == TokenKind::InterpStart {
                        self.advance(); // consume InterpStart
                        let expr = self.parse_expression(0)?;
                        segments.push(TemplateSegment::Expr(expr));
                        self.expect(TokenKind::InterpEnd)?;

                        // After InterpEnd, lexer produces another StringLit (possibly empty)
                        if self.current_kind() == TokenKind::StringLit {
                            let lit_token = self.advance_and_get();
                            if !lit_token.lexeme.is_empty() {
                                segments.push(TemplateSegment::Literal(lit_token.lexeme));
                            }
                        }
                    }

                    let span = start_span.merge(self.prev_span());
                    Ok(Expr::TemplateLit(segments, span))
                } else {
                    Ok(Expr::StringLit(token.lexeme, token.span))
                }
            }
            TokenKind::NumberLit => {
                let token = self.advance_and_get();
                let value: f64 = token.lexeme.parse().map_err(|_| {
                    format!("invalid number '{}' at {:?}", token.lexeme, token.span)
                })?;
                Ok(Expr::NumberLit(value, token.span))
            }
            TokenKind::True => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::BoolLit(true, span))
            }
            TokenKind::False => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::BoolLit(false, span))
            }
            TokenKind::None => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::NoneLit(span))
            }
            TokenKind::Ident => {
                let token = self.advance_and_get();
                // Check for function call: ident(args)
                if self.current_kind() == TokenKind::LParen {
                    self.advance();
                    let args = self.parse_call_args()?;
                    self.expect(TokenKind::RParen)?;
                    let span = token.span.merge(self.prev_span());
                    Ok(Expr::FnCall(token.lexeme, args, span))
                } else {
                    Ok(Expr::Ident(token.lexeme, token.span))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::Exec => {
                let start = self.current_span();
                self.advance(); // consume 'exec'
                self.expect(TokenKind::LBrace)?;
                self.skip_newlines();
                let prompt = self.parse_expression(0)?;
                self.skip_newlines();
                self.expect(TokenKind::RBrace)?;
                let span = start.merge(self.prev_span());
                Ok(Expr::ExecBlock(Box::new(prompt), span))
            }
            TokenKind::Recv => {
                let start = self.current_span();
                self.advance(); // consume 'recv'
                let target = self.parse_postfix()?;
                let span = start.merge(target.span());
                Ok(Expr::Recv(Box::new(target), span))
            }
            TokenKind::Retry => {
                let start = self.current_span();
                self.advance(); // consume 'retry'
                let attempts = self.parse_expression(0)?;
                self.expect(TokenKind::LBrace)?;
                let body = self.parse_block()?;
                self.expect(TokenKind::RBrace)?;
                let span = start.merge(self.prev_span());
                Ok(Expr::Retry(Box::new(attempts), body, span))
            }
            TokenKind::SelfKw => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::Ident("self".to_string(), span))
            }
            TokenKind::LBracket => {
                let start = self.current_span();
                self.advance();
                let elements = self.parse_comma_separated_exprs(TokenKind::RBracket)?;
                self.expect(TokenKind::RBracket)?;
                let span = start.merge(self.prev_span());
                Ok(Expr::ListLit(elements, span))
            }
            TokenKind::LBrace => {
                // Map literal: { key: value, key: value, ... }
                let start = self.current_span();
                self.advance(); // consume {
                self.skip_newlines();
                let mut pairs = Vec::new();
                if self.current_kind() != TokenKind::RBrace {
                    loop {
                        self.skip_newlines();
                        let key = self.parse_expression(0)?;
                        self.expect(TokenKind::Colon)?;
                        let value = self.parse_expression(0)?;
                        pairs.push((key, value));
                        self.skip_newlines();
                        if self.current_kind() != TokenKind::Comma {
                            break;
                        }
                        self.advance(); // consume comma
                    }
                }
                self.skip_newlines();
                self.expect(TokenKind::RBrace)?;
                let span = start.merge(self.prev_span());
                Ok(Expr::MapLit(pairs, span))
            }
            _ => Err(format!(
                "expected expression, found {:?} at {:?}",
                self.current_kind(),
                self.current_span()
            )),
        }
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, String> {
        self.parse_comma_separated_exprs(TokenKind::RParen)
    }

    fn parse_comma_separated_exprs(&mut self, terminator: TokenKind) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        self.skip_newlines();
        if self.current_kind() == terminator {
            return Ok(args);
        }

        loop {
            self.skip_newlines();
            args.push(self.parse_expression(0)?);
            self.skip_newlines();
            if self.current_kind() != TokenKind::Comma {
                break;
            }
            self.advance(); // consume comma
        }

        Ok(args)
    }

    // =====================================================================
    // Operator helpers
    // =====================================================================

    fn current_binop_precedence(&self) -> Option<(u8, Assoc)> {
        match self.current_kind() {
            TokenKind::Or => Some((1, Assoc::Left)),
            TokenKind::And => Some((2, Assoc::Left)),
            TokenKind::EqEq | TokenKind::BangEq => Some((3, Assoc::Left)),
            TokenKind::Lt | TokenKind::Lte | TokenKind::Gt | TokenKind::Gte => {
                Some((4, Assoc::Left))
            }
            TokenKind::Plus | TokenKind::Minus | TokenKind::PlusPlus => Some((5, Assoc::Left)),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some((6, Assoc::Left)),
            _ => Option::None,
        }
    }

    fn parse_binop(&mut self) -> Result<BinOp, String> {
        let op = match self.current_kind() {
            TokenKind::Plus => BinOp::Add,
            TokenKind::Minus => BinOp::Sub,
            TokenKind::Star => BinOp::Mul,
            TokenKind::Slash => BinOp::Div,
            TokenKind::Percent => BinOp::Mod,
            TokenKind::PlusPlus => BinOp::Concat,
            TokenKind::EqEq => BinOp::Eq,
            TokenKind::BangEq => BinOp::Neq,
            TokenKind::Lt => BinOp::Lt,
            TokenKind::Lte => BinOp::Lte,
            TokenKind::Gt => BinOp::Gt,
            TokenKind::Gte => BinOp::Gte,
            TokenKind::And => BinOp::And,
            TokenKind::Or => BinOp::Or,
            _ => {
                return Err(format!(
                    "expected binary operator, found {:?}",
                    self.current_kind()
                ));
            }
        };
        self.advance();
        Ok(op)
    }

    // =====================================================================
    // Token navigation
    // =====================================================================

    fn current_kind(&self) -> TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.kind)
            .unwrap_or(TokenKind::Eof)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::default())
    }

    fn prev_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            Span::default()
        }
    }

    fn is_at_end(&self) -> bool {
        self.current_kind() == TokenKind::Eof
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn advance_and_get(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        self.advance();
        token
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
        if self.current_kind() == kind {
            Ok(self.advance_and_get())
        } else {
            Err(format!(
                "expected {:?}, found {:?} at {:?}",
                kind,
                self.current_kind(),
                self.current_span()
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        if self.current_kind() == TokenKind::Ident {
            Ok(self.advance_and_get().lexeme)
        } else {
            Err(format!(
                "expected identifier, found {:?} at {:?}",
                self.current_kind(),
                self.current_span()
            ))
        }
    }

    fn skip_newlines(&mut self) {
        while self.current_kind() == TokenKind::Newline {
            self.advance();
        }
    }

    fn is_at_statement_end(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace | TokenKind::Semicolon
        )
    }

    fn expect_statement_end(&mut self) -> Result<(), String> {
        if self.is_at_statement_end() {
            if self.current_kind() == TokenKind::Newline
                || self.current_kind() == TokenKind::Semicolon
            {
                self.advance();
            }
            Ok(())
        } else {
            Err(format!(
                "expected end of statement, found {:?} at {:?}",
                self.current_kind(),
                self.current_span()
            ))
        }
    }

    /// Skip tokens until we find a likely statement boundary (for error recovery).
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            if self.current_kind() == TokenKind::Newline {
                self.advance();
                return;
            }
            match self.current_kind() {
                TokenKind::Let | TokenKind::Fn | TokenKind::If | TokenKind::While
                | TokenKind::For | TokenKind::Return | TokenKind::Emit
                | TokenKind::Agent | TokenKind::Tool => return,
                _ => self.advance(),
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Assoc {
    Left,
    #[allow(dead_code)]
    Right,
}

/// Convenience: parse source code directly.
pub fn parse(source: &str) -> Result<Program, Vec<String>> {
    let (tokens, lex_errors) = agentus_lexer::lexer::Lexer::new(source).tokenize();
    if !lex_errors.is_empty() {
        return Err(lex_errors);
    }
    Parser::new(tokens).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_let_string() {
        let program = parse("let x = \"hello\"").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Let(l) => {
                assert_eq!(l.name, "x");
                match &l.value {
                    Expr::StringLit(s, _) => assert_eq!(s, "hello"),
                    other => panic!("expected string lit, got {:?}", other),
                }
            }
            other => panic!("expected let, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_let_number() {
        let program = parse("let x = 42").unwrap();
        match &program.statements[0] {
            Stmt::Let(l) => match &l.value {
                Expr::NumberLit(n, _) => assert_eq!(*n, 42.0),
                other => panic!("expected number, got {:?}", other),
            },
            other => panic!("expected let, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_emit() {
        let program = parse("emit x").unwrap();
        match &program.statements[0] {
            Stmt::Emit(e) => match &e.value {
                Expr::Ident(name, _) => assert_eq!(name, "x"),
                other => panic!("expected ident, got {:?}", other),
            },
            other => panic!("expected emit, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_binary_expr() {
        let program = parse("let x = 1 + 2 * 3").unwrap();
        match &program.statements[0] {
            Stmt::Let(l) => match &l.value {
                Expr::BinOp(left, BinOp::Add, right, _) => {
                    assert!(matches!(left.as_ref(), Expr::NumberLit(1.0, _)));
                    // right should be 2 * 3
                    match right.as_ref() {
                        Expr::BinOp(l2, BinOp::Mul, r2, _) => {
                            assert!(matches!(l2.as_ref(), Expr::NumberLit(2.0, _)));
                            assert!(matches!(r2.as_ref(), Expr::NumberLit(3.0, _)));
                        }
                        other => panic!("expected mul, got {:?}", other),
                    }
                }
                other => panic!("expected binop, got {:?}", other),
            },
            other => panic!("expected let, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let program = parse("let x = foo(1, 2)").unwrap();
        match &program.statements[0] {
            Stmt::Let(l) => match &l.value {
                Expr::FnCall(name, args, _) => {
                    assert_eq!(name, "foo");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected fn call, got {:?}", other),
            },
            other => panic!("expected let, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_multiline() {
        let program = parse("let x = \"hello\"\nemit x").unwrap();
        assert_eq!(program.statements.len(), 2);
        assert!(matches!(&program.statements[0], Stmt::Let(_)));
        assert!(matches!(&program.statements[1], Stmt::Emit(_)));
    }

    #[test]
    fn test_parse_if() {
        let program = parse("if x > 0 {\n    emit x\n}").unwrap();
        assert!(matches!(&program.statements[0], Stmt::If(_)));
    }

    #[test]
    fn test_parse_fn_def() {
        let program = parse("fn add(a: num, b: num) -> num {\n    return a + b\n}").unwrap();
        match &program.statements[0] {
            Stmt::FnDef(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert!(matches!(f.return_type, Some(TypeExpr::Num)));
            }
            other => panic!("expected fn def, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_list_literal() {
        let program = parse("let xs = [1, 2, 3]").unwrap();
        match &program.statements[0] {
            Stmt::Let(l) => match &l.value {
                Expr::ListLit(elems, _) => assert_eq!(elems.len(), 3),
                other => panic!("expected list, got {:?}", other),
            },
            other => panic!("expected let, got {:?}", other),
        }
    }
}
