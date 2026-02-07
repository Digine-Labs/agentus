use agentus_common::span::Span;
use crate::token::{Token, TokenKind};

/// Lexer mode for handling string interpolation.
#[derive(Debug, Clone, Copy, PartialEq)]
enum LexMode {
    /// Normal code mode.
    Normal,
    /// Inside a string with interpolation. Tracks brace nesting depth.
    StringInterp { brace_depth: u32 },
}

/// The Agentus lexer. Converts source text into a stream of tokens.
pub struct Lexer<'src> {
    source: &'src str,
    bytes: &'src [u8],
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<String>,
    /// Mode stack for handling nested interpolation.
    mode_stack: Vec<LexMode>,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
            mode_stack: vec![LexMode::Normal],
        }
    }

    fn current_mode(&self) -> LexMode {
        *self.mode_stack.last().unwrap_or(&LexMode::Normal)
    }

    /// Tokenize the entire source, returning tokens and any errors.
    pub fn tokenize(mut self) -> (Vec<Token>, Vec<String>) {
        while !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                break;
            }

            let ch = self.peek();

            // In StringInterp mode, a closing } at depth 0 ends the interpolation
            if let LexMode::StringInterp { brace_depth } = self.current_mode() {
                if ch == b'}' && brace_depth == 0 {
                    let start = self.pos;
                    self.advance();
                    self.push_token(TokenKind::InterpEnd, start, self.pos);
                    self.mode_stack.pop();
                    // Resume string lexing
                    self.lex_string_continuation();
                    continue;
                }
            }

            match ch {
                b'\n' => {
                    let start = self.pos;
                    self.advance();
                    self.push_token(TokenKind::Newline, start, self.pos);
                }
                b'"' => self.lex_string_start(),
                b'0'..=b'9' => self.lex_number(),
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_identifier(),
                b'(' => self.single_char_token(TokenKind::LParen),
                b')' => self.single_char_token(TokenKind::RParen),
                b'{' => {
                    // Track brace depth in StringInterp mode
                    if let LexMode::StringInterp { brace_depth } = self.current_mode() {
                        *self.mode_stack.last_mut().unwrap() =
                            LexMode::StringInterp { brace_depth: brace_depth + 1 };
                    }
                    self.single_char_token(TokenKind::LBrace);
                }
                b'}' => {
                    // Decrease brace depth in StringInterp mode
                    if let LexMode::StringInterp { brace_depth } = self.current_mode() {
                        if brace_depth > 0 {
                            *self.mode_stack.last_mut().unwrap() =
                                LexMode::StringInterp { brace_depth: brace_depth - 1 };
                        }
                    }
                    self.single_char_token(TokenKind::RBrace);
                }
                b'[' => self.single_char_token(TokenKind::LBracket),
                b']' => self.single_char_token(TokenKind::RBracket),
                b',' => self.single_char_token(TokenKind::Comma),
                b':' => self.single_char_token(TokenKind::Colon),
                b';' => self.single_char_token(TokenKind::Semicolon),
                b'?' => self.single_char_token(TokenKind::Question),
                b'%' => self.single_char_token(TokenKind::Percent),
                b'*' => self.single_char_token(TokenKind::Star),
                b'.' => {
                    let start = self.pos;
                    self.advance();
                    if self.peek() == b'.' {
                        self.advance();
                        self.push_token(TokenKind::DotDot, start, self.pos);
                    } else {
                        self.push_token(TokenKind::Dot, start, self.pos);
                    }
                }
                b'+' => {
                    let start = self.pos;
                    self.advance();
                    if self.peek() == b'+' {
                        self.advance();
                        self.push_token(TokenKind::PlusPlus, start, self.pos);
                    } else {
                        self.push_token(TokenKind::Plus, start, self.pos);
                    }
                }
                b'-' => {
                    let start = self.pos;
                    self.advance();
                    if self.peek() == b'>' {
                        self.advance();
                        self.push_token(TokenKind::Arrow, start, self.pos);
                    } else {
                        self.push_token(TokenKind::Minus, start, self.pos);
                    }
                }
                b'/' => {
                    let start = self.pos;
                    self.advance();
                    self.push_token(TokenKind::Slash, start, self.pos);
                }
                b'=' => {
                    let start = self.pos;
                    self.advance();
                    if self.peek() == b'=' {
                        self.advance();
                        self.push_token(TokenKind::EqEq, start, self.pos);
                    } else if self.peek() == b'>' {
                        self.advance();
                        self.push_token(TokenKind::FatArrow, start, self.pos);
                    } else {
                        self.push_token(TokenKind::Assign, start, self.pos);
                    }
                }
                b'!' => {
                    let start = self.pos;
                    self.advance();
                    if self.peek() == b'=' {
                        self.advance();
                        self.push_token(TokenKind::BangEq, start, self.pos);
                    } else {
                        self.errors.push(format!("unexpected character '!' at position {}", start));
                        self.push_token(TokenKind::Error, start, self.pos);
                    }
                }
                b'<' => {
                    let start = self.pos;
                    self.advance();
                    if self.peek() == b'=' {
                        self.advance();
                        self.push_token(TokenKind::Lte, start, self.pos);
                    } else if self.peek() == b'-' {
                        self.advance();
                        self.push_token(TokenKind::LeftArrow, start, self.pos);
                    } else {
                        self.push_token(TokenKind::Lt, start, self.pos);
                    }
                }
                b'>' => {
                    let start = self.pos;
                    self.advance();
                    if self.peek() == b'=' {
                        self.advance();
                        self.push_token(TokenKind::Gte, start, self.pos);
                    } else {
                        self.push_token(TokenKind::Gt, start, self.pos);
                    }
                }
                _ => {
                    let start = self.pos;
                    self.advance();
                    self.errors.push(format!(
                        "unexpected character '{}' at position {}",
                        ch as char, start
                    ));
                    self.push_token(TokenKind::Error, start, self.pos);
                }
            }
        }

        self.push_token(TokenKind::Eof, self.pos, self.pos);
        (self.tokens, self.errors)
    }

    // =====================================================================
    // String lexing with interpolation
    // =====================================================================

    /// Start lexing a new string (called when we see opening `"`).
    fn lex_string_start(&mut self) {
        let start = self.pos;
        self.advance(); // consume opening "

        // Check for triple-quote
        if self.peek() == b'"' && self.peek_next() == b'"' {
            self.advance(); // second "
            self.advance(); // third "
            self.lex_triple_string(start);
            return;
        }

        self.lex_string_body(start);
    }

    /// Continue lexing a string after an interpolation ends.
    fn lex_string_continuation(&mut self) {
        let start = self.pos;
        self.lex_string_body(start);
    }

    /// Lex the body of a string, handling escape sequences and interpolation.
    /// Called both for initial string starts and after interp ends.
    fn lex_string_body(&mut self, start: usize) {
        let mut value = String::new();

        while !self.is_at_end() && self.peek() != b'"' {
            if self.peek() == b'\n' {
                self.errors.push(format!("unterminated string at position {}", start));
                self.push_token(TokenKind::Error, start, self.pos);
                return;
            }

            // Interpolation: unescaped { starts an expression
            if self.peek() == b'{' {
                // Emit accumulated string part (even if empty, for consistent parsing)
                let span = Span::new(start as u32, self.pos as u32);
                self.tokens.push(Token::new(TokenKind::StringLit, span, value));

                // Emit InterpStart
                let interp_start = self.pos;
                self.advance(); // consume {
                self.push_token(TokenKind::InterpStart, interp_start, self.pos);

                // Push StringInterp mode — the main loop will lex the expression
                self.mode_stack.push(LexMode::StringInterp { brace_depth: 0 });
                return;
            }

            if self.peek() == b'\\' {
                self.advance();
                match self.peek() {
                    b'n' => value.push('\n'),
                    b't' => value.push('\t'),
                    b'r' => value.push('\r'),
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'{' => value.push('{'),
                    b'}' => value.push('}'),
                    other => {
                        value.push('\\');
                        value.push(other as char);
                    }
                }
                self.advance();
            } else {
                value.push(self.advance() as char);
            }
        }

        if self.is_at_end() {
            self.errors.push(format!("unterminated string at position {}", start));
            self.push_token(TokenKind::Error, start, self.pos);
            return;
        }

        self.advance(); // consume closing "

        let span = Span::new(start as u32, self.pos as u32);
        self.tokens.push(Token::new(TokenKind::StringLit, span, value));
    }

    fn lex_triple_string(&mut self, start: usize) {
        let mut value = String::new();

        while !self.is_at_end() {
            if self.peek() == b'"' && self.peek_next() == b'"' {
                // Check for third "
                if self.pos + 2 < self.bytes.len() && self.bytes[self.pos + 2] == b'"' {
                    self.advance(); // first "
                    self.advance(); // second "
                    self.advance(); // third "

                    let span = Span::new(start as u32, self.pos as u32);
                    let trimmed = dedent_triple_string(&value);
                    self.tokens.push(Token::new(TokenKind::StringLit, span, trimmed));
                    return;
                }
            }
            value.push(self.advance() as char);
        }

        self.errors.push(format!("unterminated triple-quoted string at position {}", start));
        self.push_token(TokenKind::Error, start, self.pos);
    }

    // =====================================================================
    // Helpers
    // =====================================================================

    fn is_at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> u8 {
        if self.is_at_end() { 0 } else { self.bytes[self.pos] }
    }

    fn peek_next(&self) -> u8 {
        if self.pos + 1 >= self.bytes.len() { 0 } else { self.bytes[self.pos + 1] }
    }

    fn advance(&mut self) -> u8 {
        let ch = self.peek();
        self.pos += 1;
        ch
    }

    fn push_token(&mut self, kind: TokenKind, start: usize, end: usize) {
        let lexeme = self.source[start..end].to_string();
        self.tokens.push(Token::new(
            kind,
            Span::new(start as u32, end as u32),
            lexeme,
        ));
    }

    fn single_char_token(&mut self, kind: TokenKind) {
        let start = self.pos;
        self.advance();
        self.push_token(kind, start, self.pos);
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                b' ' | b'\t' | b'\r' => {
                    self.advance();
                }
                b'/' if self.peek_next() == b'/' => {
                    // Line comment: skip until newline
                    while !self.is_at_end() && self.peek() != b'\n' {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_number(&mut self) {
        let start = self.pos;
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }
        // Check for decimal point
        if self.peek() == b'.' && self.peek_next().is_ascii_digit() {
            self.advance(); // consume '.'
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }
        }
        self.push_token(TokenKind::NumberLit, start, self.pos);
    }

    fn lex_identifier(&mut self) {
        let start = self.pos;
        while !self.is_at_end() && (self.peek().is_ascii_alphanumeric() || self.peek() == b'_') {
            self.advance();
        }
        let lexeme = &self.source[start..self.pos];
        let kind = TokenKind::keyword(lexeme).unwrap_or(TokenKind::Ident);
        self.push_token(kind, start, self.pos);
    }
}

/// Dedent a triple-quoted string by removing common leading whitespace.
fn dedent_triple_string(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    // Skip first line if it's empty (common after opening """)
    let start = if lines[0].trim().is_empty() { 1 } else { 0 };
    // Skip last line if it's empty (common before closing """)
    let end = if lines.len() > 1 && lines[lines.len() - 1].trim().is_empty() {
        lines.len() - 1
    } else {
        lines.len()
    };

    if start >= end {
        return String::new();
    }

    let content_lines = &lines[start..end];

    // Find minimum indentation
    let min_indent = content_lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    // Strip common indentation
    content_lines
        .iter()
        .map(|l| {
            if l.len() >= min_indent {
                &l[min_indent..]
            } else {
                l.trim()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<Token> {
        let (tokens, errors) = Lexer::new(source).tokenize();
        assert!(errors.is_empty(), "lexer errors: {:?}", errors);
        tokens
    }

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source).into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn test_let_statement() {
        assert_eq!(
            kinds("let x = \"hello\""),
            vec![
                TokenKind::Let,
                TokenKind::Ident,
                TokenKind::Assign,
                TokenKind::StringLit,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_emit() {
        assert_eq!(
            kinds("emit x"),
            vec![TokenKind::Emit, TokenKind::Ident, TokenKind::Eof]
        );
    }

    #[test]
    fn test_number() {
        let tokens = lex("42 3.14");
        assert_eq!(tokens[0].kind, TokenKind::NumberLit);
        assert_eq!(tokens[0].lexeme, "42");
        assert_eq!(tokens[1].kind, TokenKind::NumberLit);
        assert_eq!(tokens[1].lexeme, "3.14");
    }

    #[test]
    fn test_operators() {
        assert_eq!(
            kinds("+ - * / % ++ == != < <= > >= = => -> <-"),
            vec![
                TokenKind::Plus, TokenKind::Minus, TokenKind::Star,
                TokenKind::Slash, TokenKind::Percent, TokenKind::PlusPlus,
                TokenKind::EqEq, TokenKind::BangEq, TokenKind::Lt,
                TokenKind::Lte, TokenKind::Gt, TokenKind::Gte,
                TokenKind::Assign, TokenKind::FatArrow, TokenKind::Arrow,
                TokenKind::LeftArrow, TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            kinds("agent fn let return if else emit exec spawn"),
            vec![
                TokenKind::Agent, TokenKind::Fn, TokenKind::Let,
                TokenKind::Return, TokenKind::If, TokenKind::Else,
                TokenKind::Emit, TokenKind::Exec, TokenKind::Spawn,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_comments_skipped() {
        assert_eq!(
            kinds("let x = 5 // this is a comment\nemit x"),
            vec![
                TokenKind::Let, TokenKind::Ident, TokenKind::Assign,
                TokenKind::NumberLit, TokenKind::Newline,
                TokenKind::Emit, TokenKind::Ident, TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_escapes() {
        let tokens = lex(r#""hello\nworld""#);
        assert_eq!(tokens[0].kind, TokenKind::StringLit);
        assert_eq!(tokens[0].lexeme, "hello\nworld");
    }

    #[test]
    fn test_braces_and_brackets() {
        assert_eq!(
            kinds("( ) { } [ ]"),
            vec![
                TokenKind::LParen, TokenKind::RParen,
                TokenKind::LBrace, TokenKind::RBrace,
                TokenKind::LBracket, TokenKind::RBracket,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_multiline() {
        assert_eq!(
            kinds("let x = 5\nemit x"),
            vec![
                TokenKind::Let, TokenKind::Ident, TokenKind::Assign,
                TokenKind::NumberLit, TokenKind::Newline,
                TokenKind::Emit, TokenKind::Ident, TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_dedent_triple_string() {
        let input = "\n        hello\n        world\n    ";
        let result = dedent_triple_string(input);
        assert_eq!(result, "hello\nworld");
    }

    // String interpolation tests

    #[test]
    fn test_string_interpolation_simple() {
        let k = kinds(r#""hello {name}""#);
        assert_eq!(
            k,
            vec![
                TokenKind::StringLit,   // "hello "
                TokenKind::InterpStart, // {
                TokenKind::Ident,       // name
                TokenKind::InterpEnd,   // }
                TokenKind::StringLit,   // "" (empty trailing)
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_interpolation_values() {
        let tokens = lex(r#""hello {name}!""#);
        assert_eq!(tokens[0].lexeme, "hello "); // before {
        assert_eq!(tokens[2].lexeme, "name");   // identifier
        assert_eq!(tokens[4].lexeme, "!");       // after }
    }

    #[test]
    fn test_string_interpolation_multiple() {
        let k = kinds(r#""{a} and {b}""#);
        assert_eq!(
            k,
            vec![
                TokenKind::StringLit,   // "" (empty before first interp)
                TokenKind::InterpStart,
                TokenKind::Ident,       // a
                TokenKind::InterpEnd,
                TokenKind::StringLit,   // " and "
                TokenKind::InterpStart,
                TokenKind::Ident,       // b
                TokenKind::InterpEnd,
                TokenKind::StringLit,   // "" (empty trailing)
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_interpolation_expr() {
        let k = kinds(r#""sum: {a + b}""#);
        assert_eq!(
            k,
            vec![
                TokenKind::StringLit,   // "sum: "
                TokenKind::InterpStart,
                TokenKind::Ident,       // a
                TokenKind::Plus,        // +
                TokenKind::Ident,       // b
                TokenKind::InterpEnd,
                TokenKind::StringLit,   // "" (empty trailing)
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_no_interpolation() {
        // Plain string without interpolation
        let k = kinds(r#""hello world""#);
        assert_eq!(k, vec![TokenKind::StringLit, TokenKind::Eof]);
    }

    #[test]
    fn test_string_escaped_braces() {
        // Escaped braces should not trigger interpolation
        let tokens = lex(r#""hello \{world\}""#);
        assert_eq!(tokens[0].kind, TokenKind::StringLit);
        assert_eq!(tokens[0].lexeme, "hello {world}");
    }
}
