//! End-to-end integration tests: source code → compile → execute → check output.
//!
//! These tests exercise the full pipeline (lexer → parser → sema → codegen → runtime).

use agentus_codegen::compiler::compile;
use agentus_runtime::host::{EchoHost, HostInterface};
use agentus_runtime::value::Value;
use agentus_runtime::vm::{SilentHandler, VM};

/// Helper: compile source, run VM, return collected outputs as strings.
fn run(source: &str) -> Vec<String> {
    let module = compile(source).unwrap_or_else(|e| panic!("compile error: {}", e));
    let mut vm = VM::new(module).with_output(Box::new(SilentHandler));
    vm.run().unwrap_or_else(|e| panic!("runtime error: {}", e));
    vm.get_outputs().iter().map(|v| v.to_string()).collect()
}

/// Helper: compile source, run VM, return raw Value outputs.
fn run_values(source: &str) -> Vec<Value> {
    let module = compile(source).unwrap_or_else(|e| panic!("compile error: {}", e));
    let mut vm = VM::new(module).with_output(Box::new(SilentHandler));
    vm.run().unwrap_or_else(|e| panic!("runtime error: {}", e));
    vm.get_outputs().to_vec()
}

/// Helper: compile source and expect a compile error containing the given substring.
fn expect_compile_error(source: &str, expected: &str) {
    let result = compile(source);
    assert!(result.is_err(), "expected compile error, got Ok");
    let err = result.unwrap_err();
    assert!(
        err.contains(expected),
        "expected error containing '{}', got: {}",
        expected,
        err
    );
}

// ===================================================================
// Basic literals and let/emit
// ===================================================================

#[test]
fn test_hello_world() {
    let out = run("let greeting = \"Hello Agentus!\"\nemit greeting");
    assert_eq!(out, vec!["Hello Agentus!"]);
}

#[test]
fn test_number_literal() {
    let out = run("emit 42");
    assert_eq!(out, vec!["42"]);
}

#[test]
fn test_string_literal() {
    let out = run("emit \"hello\"");
    assert_eq!(out, vec!["hello"]);
}

#[test]
fn test_bool_literals() {
    let out = run("emit true\nemit false");
    assert_eq!(out, vec!["true", "false"]);
}

#[test]
fn test_none_literal() {
    let out = run("emit none");
    assert_eq!(out, vec!["none"]);
}

#[test]
fn test_multiple_lets() {
    let out = run("let a = 1\nlet b = 2\nlet c = 3\nemit a\nemit b\nemit c");
    assert_eq!(out, vec!["1", "2", "3"]);
}

// ===================================================================
// Arithmetic
// ===================================================================

#[test]
fn test_addition() {
    let out = run("emit 3 + 7");
    assert_eq!(out, vec!["10"]);
}

#[test]
fn test_subtraction() {
    let out = run("emit 10 - 4");
    assert_eq!(out, vec!["6"]);
}

#[test]
fn test_multiplication() {
    let out = run("emit 6 * 5");
    assert_eq!(out, vec!["30"]);
}

#[test]
fn test_division() {
    let out = run("emit 15 / 3");
    assert_eq!(out, vec!["5"]);
}

#[test]
fn test_modulo() {
    let out = run("emit 17 % 5");
    assert_eq!(out, vec!["2"]);
}

#[test]
fn test_operator_precedence() {
    // 2 + 3 * 4 = 14 (not 20)
    let out = run("emit 2 + 3 * 4");
    assert_eq!(out, vec!["14"]);
}

#[test]
fn test_complex_arithmetic() {
    let out = run("let x = 10\nlet y = 3\nemit x + y\nemit x * y\nemit x > y");
    assert_eq!(out, vec!["13", "30", "true"]);
}

#[test]
fn test_negation() {
    let out = run("let x = 5\nemit -x");
    assert_eq!(out, vec!["-5"]);
}

// ===================================================================
// Comparisons
// ===================================================================

#[test]
fn test_greater_than() {
    let out = run("emit 5 > 3");
    assert_eq!(out, vec!["true"]);
}

#[test]
fn test_less_than() {
    let out = run("emit 5 < 3");
    assert_eq!(out, vec!["false"]);
}

#[test]
fn test_equality() {
    let out = run("emit 5 == 5");
    assert_eq!(out, vec!["true"]);
}

#[test]
fn test_inequality() {
    let out = run("emit 5 != 3");
    assert_eq!(out, vec!["true"]);
}

#[test]
fn test_lte_gte() {
    let out = run("emit 5 >= 5\nemit 5 <= 5\nemit 4 >= 5\nemit 6 <= 5");
    assert_eq!(out, vec!["true", "true", "false", "false"]);
}

// ===================================================================
// Boolean logic
// ===================================================================

#[test]
fn test_and_or() {
    let out = run("emit true and false\nemit true or false");
    assert_eq!(out, vec!["false", "true"]);
}

#[test]
fn test_not() {
    let out = run("emit not true\nemit not false");
    assert_eq!(out, vec!["false", "true"]);
}

// ===================================================================
// String concatenation
// ===================================================================

#[test]
fn test_string_concat() {
    let out = run("emit \"hello\" ++ \" world\"");
    assert_eq!(out, vec!["hello world"]);
}

// ===================================================================
// If / else
// ===================================================================

#[test]
fn test_if_true_branch() {
    let out = run("let x = 10\nif x > 5 {\n    emit \"big\"\n}");
    assert_eq!(out, vec!["big"]);
}

#[test]
fn test_if_false_branch() {
    let out = run("let x = 3\nif x > 5 {\n    emit \"big\"\n}");
    assert!(out.is_empty());
}

#[test]
fn test_if_else() {
    let out = run("let x = 3\nif x > 5 {\n    emit \"big\"\n} else {\n    emit \"small\"\n}");
    assert_eq!(out, vec!["small"]);
}

#[test]
fn test_if_with_arithmetic() {
    let src = r#"
let score = 85
if score >= 90 {
    emit "A"
} else {
    if score >= 80 {
        emit "B"
    } else {
        emit "C"
    }
}
"#;
    let out = run(src);
    assert_eq!(out, vec!["B"]);
}

// ===================================================================
// While loops
// ===================================================================

#[test]
fn test_while_basic() {
    let src = r#"
let i = 0
while i < 5 {
    emit i
    i = i + 1
}
"#;
    let out = run(src);
    assert_eq!(out, vec!["0", "1", "2", "3", "4"]);
}

#[test]
fn test_while_never_executes() {
    let src = r#"
let i = 10
while i < 5 {
    emit i
    i = i + 1
}
emit "done"
"#;
    let out = run(src);
    assert_eq!(out, vec!["done"]);
}

#[test]
fn test_while_sum() {
    let src = r#"
let sum = 0
let i = 1
while i <= 10 {
    sum = sum + i
    i = i + 1
}
emit sum
"#;
    let out = run(src);
    assert_eq!(out, vec!["55"]);
}

// ===================================================================
// For loops
// ===================================================================

#[test]
fn test_for_basic() {
    let src = r#"
let items = ["apple", "banana", "cherry"]
for item in items {
    emit item
}
"#;
    let out = run(src);
    assert_eq!(out, vec!["apple", "banana", "cherry"]);
}

#[test]
fn test_for_empty_list() {
    let src = r#"
let items = []
for item in items {
    emit item
}
emit "done"
"#;
    let out = run(src);
    assert_eq!(out, vec!["done"]);
}

#[test]
fn test_for_with_numbers() {
    let src = r#"
let nums = [1, 2, 3, 4, 5]
let sum = 0
for n in nums {
    sum = sum + n
}
emit sum
"#;
    let out = run(src);
    assert_eq!(out, vec!["15"]);
}

// ===================================================================
// Function definitions and calls
// ===================================================================

#[test]
fn test_function_basic() {
    let src = r#"
fn double(x: num) -> num {
    return x * 2
}
emit double(5)
"#;
    let out = run(src);
    assert_eq!(out, vec!["10"]);
}

#[test]
fn test_function_two_params() {
    let src = r#"
fn add(a: num, b: num) -> num {
    return a + b
}
emit add(3, 7)
"#;
    let out = run(src);
    assert_eq!(out, vec!["10"]);
}

#[test]
fn test_function_with_variable_args() {
    let src = r#"
fn multiply(a: num, b: num) -> num {
    return a * b
}
let x = 6
let y = 7
emit multiply(x, y)
"#;
    let out = run(src);
    assert_eq!(out, vec!["42"]);
}

#[test]
fn test_function_multiple_calls() {
    let src = r#"
fn square(x: num) -> num {
    return x * x
}
emit square(3)
emit square(4)
emit square(5)
"#;
    let out = run(src);
    assert_eq!(out, vec!["9", "16", "25"]);
}

#[test]
fn test_function_string_return() {
    let src = r#"
fn greet(name: str) -> str {
    return "Hello, " ++ name ++ "!"
}
emit greet("World")
"#;
    let out = run(src);
    assert_eq!(out, vec!["Hello, World!"]);
}

// ===================================================================
// String interpolation
// ===================================================================

#[test]
fn test_interpolation_simple() {
    let src = r#"
let name = "World"
emit "Hello, {name}!"
"#;
    let out = run(src);
    assert_eq!(out, vec!["Hello, World!"]);
}

#[test]
fn test_interpolation_expression() {
    let src = r#"
let a = 10
let b = 32
emit "The sum of {a} and {b} is {a + b}"
"#;
    let out = run(src);
    assert_eq!(out, vec!["The sum of 10 and 32 is 42"]);
}

#[test]
fn test_interpolation_with_function_call() {
    let src = r#"
fn double(x: num) -> num {
    return x * 2
}
let a = 10
emit "Double of {a} is {double(a)}"
"#;
    let out = run(src);
    assert_eq!(out, vec!["Double of 10 is 20"]);
}

#[test]
fn test_interpolation_only_expr() {
    let src = r#"
let x = 42
emit "{x}"
"#;
    let out = run(src);
    assert_eq!(out, vec!["42"]);
}

#[test]
fn test_no_interpolation() {
    let out = run("emit \"plain string\"");
    assert_eq!(out, vec!["plain string"]);
}

// ===================================================================
// Variable assignment
// ===================================================================

#[test]
fn test_variable_reassignment() {
    let src = r#"
let x = 1
emit x
x = 2
emit x
x = x + 10
emit x
"#;
    let out = run(src);
    assert_eq!(out, vec!["1", "2", "12"]);
}

// ===================================================================
// Combined / complex programs
// ===================================================================

#[test]
fn test_fibonacci() {
    let src = r#"
let a = 0
let b = 1
let i = 0
while i < 8 {
    emit a
    let temp = a + b
    a = b
    b = temp
    i = i + 1
}
"#;
    let out = run(src);
    assert_eq!(out, vec!["0", "1", "1", "2", "3", "5", "8", "13"]);
}

#[test]
fn test_countdown() {
    let src = r#"
let n = 5
while n > 0 {
    emit n
    n = n - 1
}
emit "Go!"
"#;
    let out = run(src);
    assert_eq!(out, vec!["5", "4", "3", "2", "1", "Go!"]);
}

#[test]
fn test_nested_if_in_while() {
    let src = r#"
let i = 0
while i < 6 {
    if i % 2 == 0 {
        emit i
    }
    i = i + 1
}
"#;
    let out = run(src);
    assert_eq!(out, vec!["0", "2", "4"]);
}

#[test]
fn test_for_with_if() {
    let src = r#"
let items = [1, 2, 3, 4, 5, 6]
for item in items {
    if item > 3 {
        emit item
    }
}
"#;
    let out = run(src);
    assert_eq!(out, vec!["4", "5", "6"]);
}

#[test]
fn test_function_with_if() {
    let src = r#"
fn abs(x: num) -> num {
    if x < 0 {
        return -x
    }
    return x
}
emit abs(5)
emit abs(-3)
"#;
    let out = run(src);
    assert_eq!(out, vec!["5", "3"]);
}

// ===================================================================
// Error cases
// ===================================================================

#[test]
fn test_undefined_variable_error() {
    expect_compile_error("emit x", "undefined variable");
}

#[test]
fn test_undefined_function_error() {
    expect_compile_error("emit foo()", "undefined function");
}

// ===================================================================
// Value type checks
// ===================================================================

#[test]
fn test_output_types() {
    let vals = run_values("emit 42\nemit \"hello\"\nemit true\nemit none");
    assert_eq!(vals[0], Value::Num(42.0));
    assert_eq!(vals[1], Value::from_str("hello"));
    assert_eq!(vals[2], Value::Bool(true));
    assert_eq!(vals[3], Value::None);
}

// ===================================================================
// Helper: compile and run with a HostInterface
// ===================================================================

fn run_with_host(source: &str, host: Box<dyn HostInterface>) -> Vec<String> {
    let module = compile(source).unwrap_or_else(|e| panic!("compile error: {}", e));
    let mut vm = VM::new(module)
        .with_output(Box::new(SilentHandler))
        .with_host(host);
    vm.run().unwrap_or_else(|e| panic!("runtime error: {}", e));
    vm.get_outputs().iter().map(|v| v.to_string()).collect()
}

// ===================================================================
// Tool tests
// ===================================================================

#[test]
fn test_tool_basic_call() {
    let src = r#"
tool greet {
    description { "Greet someone" }
    param name: str
    returns str
}
let result = greet("Alice")
emit result
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["greet(name=Alice)"]);
}

#[test]
fn test_tool_two_params() {
    let src = r#"
tool get_weather {
    description { "Get weather for a location" }
    param location: str
    param units: str
    returns str
}
let result = get_weather("London", "celsius")
emit result
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["get_weather(location=London, units=celsius)"]);
}

#[test]
fn test_tool_default_params() {
    let src = r#"
tool get_weather {
    description { "Get weather for a location" }
    param location: str
    param units: str = "celsius"
    returns str
}
let result = get_weather("London")
emit result
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["get_weather(location=London, units=celsius)"]);
}

#[test]
fn test_tool_default_override() {
    let src = r#"
tool get_weather {
    description { "Get weather for a location" }
    param location: str
    param units: str = "celsius"
    returns str
}
let result = get_weather("New York", "fahrenheit")
emit result
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["get_weather(location=New York, units=fahrenheit)"]);
}

#[test]
fn test_tool_no_description() {
    let src = r#"
tool ping {
    param host: str
    returns str
}
emit ping("localhost")
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["ping(host=localhost)"]);
}

#[test]
fn test_tool_with_variable_args() {
    let src = r#"
tool send_email {
    description { "Send an email" }
    param to: str
    param subject: str
    param body: str
    returns str
}
let recipient = "alice@example.com"
let subj = "Hello"
let msg = "How are you?"
emit send_email(recipient, subj, msg)
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["send_email(to=alice@example.com, subject=Hello, body=How are you?)"]);
}

#[test]
fn test_tool_multiple_calls() {
    let src = r#"
tool add {
    param a: num
    param b: num
    returns num
}
emit add(1, 2)
emit add(10, 20)
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["add(a=1, b=2)", "add(a=10, b=20)"]);
}

#[test]
fn test_tool_with_agent() {
    let src = r#"
tool get_weather {
    description { "Get current weather" }
    param location: str
    returns str
}

agent WeatherBot {
    model = "gpt-4o"

    memory {
        last_location: str = "unknown"
    }

    fn check_weather(city: str) -> str {
        self.last_location = city
        return get_weather(city)
    }
}

let bot = WeatherBot()
emit bot.check_weather("Tokyo")
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["get_weather(location=Tokyo)"]);
}

#[test]
fn test_tool_result_in_variable() {
    let src = r#"
tool fetch_data {
    description { "Fetch data from a source" }
    param source: str
    returns str
}
let data = fetch_data("database")
emit "Got: " ++ data
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["Got: fetch_data(source=database)"]);
}

#[test]
fn test_undefined_tool_error() {
    expect_compile_error(
        "emit unknown_tool(\"test\")",
        "undefined function or tool",
    );
}

// ===================================================================
// Agent core tests
// ===================================================================

#[test]
fn test_agent_instantiation() {
    let src = r#"
agent Greeter {
    model = "gpt-4o"
}
let g = Greeter()
emit g
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["<agent:1>"]);
}

#[test]
fn test_agent_method_simple_return() {
    let src = r#"
agent Calculator {
    fn add(a: num, b: num) -> num {
        return a + b
    }
}
let c = Calculator()
emit c.add(3, 7)
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["10"]);
}

#[test]
fn test_agent_memory_persistence() {
    let src = r#"
agent Counter {
    memory {
        count: num = 0
    }

    fn increment() -> num {
        self.count = self.count + 1
        return self.count
    }
}
let c = Counter()
emit c.increment()
emit c.increment()
emit c.increment()
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["1", "2", "3"]);
}

#[test]
fn test_exec_block_echo() {
    let src = r#"
let result = exec { "What is 2+2?" }
emit result
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["What is 2+2?"]);
}

#[test]
fn test_exec_block_inline() {
    let src = r#"
emit exec { "hello from exec" }
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["hello from exec"]);
}

#[test]
fn test_multiple_agent_instances() {
    let src = r#"
agent Counter {
    memory {
        count: num = 0
    }

    fn increment() -> num {
        self.count = self.count + 1
        return self.count
    }
}
let a = Counter()
let b = Counter()
emit a.increment()
emit a.increment()
emit b.increment()
emit a.increment()
emit b.increment()
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["1", "2", "1", "3", "2"]);
}

#[test]
fn test_agent_with_model_and_prompt() {
    let src = r#"
agent Assistant {
    model = "gpt-4o"
    system prompt { "You are helpful." }

    memory {
        count: num = 0
    }

    fn get_count() -> num {
        return self.count
    }
}
let a = Assistant()
emit a.get_count()
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["0"]);
}

#[test]
fn test_agent_memory_default_string() {
    let src = r#"
agent Bot {
    memory {
        name: str = "unnamed"
    }

    fn get_name() -> str {
        return self.name
    }
}
let b = Bot()
emit b.get_name()
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["unnamed"]);
}

#[test]
fn test_agent_method_no_params() {
    let src = r#"
agent Ping {
    fn pong() -> str {
        return "pong"
    }
}
let p = Ping()
emit p.pong()
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["pong"]);
}

#[test]
fn test_agent_full_example() {
    let src = r#"
agent Counter {
    model = "gpt-4o"
    system prompt { "You are a counting assistant." }

    memory {
        count: num = 0
    }

    fn increment() -> num {
        self.count = self.count + 1
        return self.count
    }

    fn get_count() -> num {
        return self.count
    }
}

let c = Counter()
emit c.increment()
emit c.increment()
emit c.increment()
emit c.get_count()

let result = exec { "What is 2+2?" }
emit result
"#;
    let out = run_with_host(src, Box::new(EchoHost));
    assert_eq!(out, vec!["1", "2", "3", "3", "What is 2+2?"]);
}

// ===================================================================
// Phase 5: Send/Recv (agent message passing)
// ===================================================================

#[test]
fn test_send_recv_basic() {
    let src = r#"
agent Box {
    memory { }
}
let b = Box()
send b, "hello"
let msg = recv b
emit msg
"#;
    let out = run(src);
    assert_eq!(out, vec!["hello"]);
}

#[test]
fn test_recv_empty() {
    let src = r#"
agent Box {
    memory { }
}
let b = Box()
let msg = recv b
emit msg
"#;
    let out = run(src);
    assert_eq!(out, vec!["none"]);
}

#[test]
fn test_send_recv_ordering() {
    let src = r#"
agent Box {
    memory { }
}
let b = Box()
send b, "first"
send b, "second"
emit recv b
emit recv b
"#;
    let out = run(src);
    assert_eq!(out, vec!["first", "second"]);
}

#[test]
fn test_send_recv_multiple_agents() {
    let src = r#"
agent Box {
    memory { }
}
let a = Box()
let b = Box()
send a, "for-a"
send b, "for-b"
emit recv a
emit recv b
"#;
    let out = run(src);
    assert_eq!(out, vec!["for-a", "for-b"]);
}

#[test]
fn test_send_different_types() {
    let src = r#"
agent Box {
    memory { }
}
let b = Box()
send b, 42
send b, true
send b, "text"
emit recv b
emit recv b
emit recv b
"#;
    let out = run(src);
    assert_eq!(out, vec!["42", "true", "text"]);
}
