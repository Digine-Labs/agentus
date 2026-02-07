use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

/// Runtime value representation.
///
/// Strings are Rc for cheap cloning (prompts/responses are large).
/// Collections are Rc<RefCell<...>> for shared mutability.
#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Num(f64),
    Str(Rc<String>),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    AgentHandle(u64),
    Error(Rc<String>),
    /// Internal iterator state: (source items, current index).
    Iterator(Rc<RefCell<(Vec<Value>, usize)>>),
}

impl Value {
    pub fn from_str(s: &str) -> Self {
        Value::Str(Rc::new(s.to_string()))
    }

    pub fn from_string(s: String) -> Self {
        Value::Str(Rc::new(s))
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::None => false,
            Value::Bool(b) => *b,
            Value::Num(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.borrow().is_empty(),
            Value::Map(m) => !m.borrow().is_empty(),
            Value::AgentHandle(_) => true,
            Value::Error(_) => false,
            Value::Iterator(_) => true,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s.as_str()),
            _ => Option::None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => Option::None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => Option::None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::None => write!(f, "none"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Num(n) => {
                if *n == (*n as i64 as f64) {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::List(l) => {
                let items = l.borrow();
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                let items = m.borrow();
                write!(f, "{{")?;
                for (i, (k, v)) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::AgentHandle(id) => write!(f, "<agent:{}>", id),
            Value::Error(e) => write!(f, "<error: {}>", e),
            Value::Iterator(_) => write!(f, "<iterator>"),
        }
    }
}

impl Value {
    /// Serialize this value to a JSON string.
    pub fn to_json(&self) -> String {
        match self {
            Value::None => "null".to_string(),
            Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
            Value::Num(n) => {
                if *n == (*n as i64 as f64) {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Str(s) => {
                let mut out = String::with_capacity(s.len() + 2);
                out.push('"');
                for ch in s.chars() {
                    match ch {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c => out.push(c),
                    }
                }
                out.push('"');
                out
            }
            Value::List(l) => {
                let items = l.borrow();
                let parts: Vec<String> = items.iter().map(|v| v.to_json()).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Map(m) => {
                let items = m.borrow();
                let parts: Vec<String> = items.iter().map(|(k, v)| {
                    let key_escaped = Value::from_str(k).to_json();
                    format!("{}: {}", key_escaped, v.to_json())
                }).collect();
                format!("{{{}}}", parts.join(", "))
            }
            Value::AgentHandle(id) => format!("\"<agent:{}>\"", id),
            Value::Error(e) => {
                let escaped = Value::from_str(e).to_json();
                escaped
            }
            Value::Iterator(_) => "null".to_string(),
        }
    }

    /// Parse a JSON string into a Value.
    /// Returns Value::None on parse failure.
    pub fn parse_json(input: &str) -> Result<Value, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("empty JSON input".to_string());
        }
        let bytes = trimmed.as_bytes();
        let (val, rest) = json_parse_value(bytes)?;
        let rest = skip_ws(rest);
        if !rest.is_empty() {
            return Err("trailing content after JSON value".to_string());
        }
        Ok(val)
    }
}

// =====================================================================
// Simple JSON parser (no external dependencies)
// =====================================================================

fn skip_ws(input: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < input.len() && matches!(input[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    &input[i..]
}

fn json_parse_value(input: &[u8]) -> Result<(Value, &[u8]), String> {
    let input = skip_ws(input);
    if input.is_empty() {
        return Err("unexpected end of JSON".to_string());
    }
    match input[0] {
        b'"' => json_parse_string(input),
        b'{' => json_parse_object(input),
        b'[' => json_parse_array(input),
        b't' => json_parse_true(input),
        b'f' => json_parse_false(input),
        b'n' => json_parse_null(input),
        b'-' | b'0'..=b'9' => json_parse_number(input),
        c => Err(format!("unexpected character '{}' in JSON", c as char)),
    }
}

fn json_parse_string(input: &[u8]) -> Result<(Value, &[u8]), String> {
    if input.is_empty() || input[0] != b'"' {
        return Err("expected '\"'".to_string());
    }
    let mut i = 1;
    let mut s = String::new();
    while i < input.len() {
        match input[i] {
            b'"' => {
                return Ok((Value::from_string(s), &input[i + 1..]));
            }
            b'\\' => {
                i += 1;
                if i >= input.len() {
                    return Err("unterminated string escape".to_string());
                }
                match input[i] {
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    b'/' => s.push('/'),
                    b'n' => s.push('\n'),
                    b'r' => s.push('\r'),
                    b't' => s.push('\t'),
                    b'u' => {
                        // Unicode escape: \uXXXX
                        if i + 4 >= input.len() {
                            return Err("incomplete unicode escape".to_string());
                        }
                        let hex = std::str::from_utf8(&input[i + 1..i + 5])
                            .map_err(|_| "invalid unicode escape".to_string())?;
                        let code = u32::from_str_radix(hex, 16)
                            .map_err(|_| "invalid unicode escape".to_string())?;
                        if let Some(ch) = char::from_u32(code) {
                            s.push(ch);
                        }
                        i += 4;
                    }
                    c => {
                        s.push('\\');
                        s.push(c as char);
                    }
                }
            }
            c => s.push(c as char),
        }
        i += 1;
    }
    Err("unterminated string".to_string())
}

fn json_parse_object(input: &[u8]) -> Result<(Value, &[u8]), String> {
    let mut rest = skip_ws(&input[1..]); // skip '{'
    let mut map = HashMap::new();

    if !rest.is_empty() && rest[0] == b'}' {
        return Ok((Value::Map(Rc::new(RefCell::new(map))), &rest[1..]));
    }

    loop {
        rest = skip_ws(rest);
        let (key_val, after_key) = json_parse_string(rest)?;
        let key = match key_val {
            Value::Str(s) => (*s).clone(),
            _ => return Err("object key must be a string".to_string()),
        };
        rest = skip_ws(after_key);
        if rest.is_empty() || rest[0] != b':' {
            return Err("expected ':' in object".to_string());
        }
        rest = skip_ws(&rest[1..]);
        let (val, after_val) = json_parse_value(rest)?;
        map.insert(key, val);
        rest = skip_ws(after_val);
        if rest.is_empty() {
            return Err("unterminated object".to_string());
        }
        match rest[0] {
            b'}' => return Ok((Value::Map(Rc::new(RefCell::new(map))), &rest[1..])),
            b',' => rest = &rest[1..],
            _ => return Err("expected ',' or '}' in object".to_string()),
        }
    }
}

fn json_parse_array(input: &[u8]) -> Result<(Value, &[u8]), String> {
    let mut rest = skip_ws(&input[1..]); // skip '['
    let mut items = Vec::new();

    if !rest.is_empty() && rest[0] == b']' {
        return Ok((Value::List(Rc::new(RefCell::new(items))), &rest[1..]));
    }

    loop {
        rest = skip_ws(rest);
        let (val, after_val) = json_parse_value(rest)?;
        items.push(val);
        rest = skip_ws(after_val);
        if rest.is_empty() {
            return Err("unterminated array".to_string());
        }
        match rest[0] {
            b']' => return Ok((Value::List(Rc::new(RefCell::new(items))), &rest[1..])),
            b',' => rest = &rest[1..],
            _ => return Err("expected ',' or ']' in array".to_string()),
        }
    }
}

fn json_parse_number(input: &[u8]) -> Result<(Value, &[u8]), String> {
    let mut i = 0;
    if i < input.len() && input[i] == b'-' {
        i += 1;
    }
    while i < input.len() && input[i].is_ascii_digit() {
        i += 1;
    }
    if i < input.len() && input[i] == b'.' {
        i += 1;
        while i < input.len() && input[i].is_ascii_digit() {
            i += 1;
        }
    }
    // Handle exponent
    if i < input.len() && (input[i] == b'e' || input[i] == b'E') {
        i += 1;
        if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
            i += 1;
        }
        while i < input.len() && input[i].is_ascii_digit() {
            i += 1;
        }
    }
    let num_str = std::str::from_utf8(&input[..i])
        .map_err(|_| "invalid number".to_string())?;
    let n: f64 = num_str.parse()
        .map_err(|_| format!("cannot parse number: {}", num_str))?;
    Ok((Value::Num(n), &input[i..]))
}

fn json_parse_true(input: &[u8]) -> Result<(Value, &[u8]), String> {
    if input.len() >= 4 && &input[..4] == b"true" {
        Ok((Value::Bool(true), &input[4..]))
    } else {
        Err("expected 'true'".to_string())
    }
}

fn json_parse_false(input: &[u8]) -> Result<(Value, &[u8]), String> {
    if input.len() >= 5 && &input[..5] == b"false" {
        Ok((Value::Bool(false), &input[5..]))
    } else {
        Err("expected 'false'".to_string())
    }
}

fn json_parse_null(input: &[u8]) -> Result<(Value, &[u8]), String> {
    if input.len() >= 4 && &input[..4] == b"null" {
        Ok((Value::None, &input[4..]))
    } else {
        Err("expected 'null'".to_string())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::None, Value::None) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Num(a), Value::Num(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            _ => false,
        }
    }
}
