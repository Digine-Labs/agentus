# Agentus Development Workflow

## Quick Start

```bash
# Build everything
make build

# Run all tests (119 currently)
make test

# Fast verification (build + test + run 2 examples)
make smoke

# Run a specific example
make run-example EX=hello.ags
make run-example EX=agent_basic.ags

# Format code
make fmt

# Lint
make lint
```

## Running Examples

All example programs are in the `examples/` directory:

```bash
cargo run -p agentus-cli -- exec examples/hello.ags
cargo run -p agentus-cli -- exec examples/arithmetic.ags
cargo run -p agentus-cli -- exec examples/control_flow.ags
cargo run -p agentus-cli -- exec examples/functions.ags
cargo run -p agentus-cli -- exec examples/for_loop.ags
cargo run -p agentus-cli -- exec examples/while_loop.ags
cargo run -p agentus-cli -- exec examples/interpolation.ags
cargo run -p agentus-cli -- exec examples/agent_basic.ags
cargo run -p agentus-cli -- exec examples/tools.ags
```

The CLI uses `EchoHost` by default, which means:
- `exec { "prompt" }` returns the prompt string itself
- Tool calls return a formatted string like `tool_name(param1=val1, param2=val2)`

## Adding Tests

### Integration Tests (preferred for end-to-end features)

File: `crates/agentus-codegen/tests/end_to_end.rs`

```rust
// Basic test — compile + run, check emit output
#[test]
fn test_my_feature() {
    let out = run(r#"
        let x = 42
        emit x
    "#);
    assert_eq!(out, vec!["42"]);
}

// Test with custom host (for agent/tool features)
#[test]
fn test_my_agent_feature() {
    let out = run_with_host(r#"
        // agent code here
    "#, Box::new(EchoHost));
    assert_eq!(out, vec!["expected output"]);
}

// Test raw values
#[test]
fn test_raw_values() {
    let vals = run_values(r#"
        emit 42
        emit true
    "#);
    assert_eq!(vals[0], Value::Num(42.0));
    assert_eq!(vals[1], Value::Bool(true));
}

// Test that compilation fails with expected error
#[test]
fn test_error_case() {
    expect_compile_error(r#"
        emit undefined_var
    "#, "undefined variable");
}
```

### Unit Tests (for specific crate internals)

Add `#[cfg(test)] mod tests { ... }` blocks within the relevant source file.

Example locations:
- Lexer: `crates/agentus-lexer/src/lexer.rs`
- Parser: `crates/agentus-parser/src/parser.rs`
- Sema: `crates/agentus-sema/src/resolver.rs`
- IR: `crates/agentus-ir/src/instruction.rs`, `crates/agentus-ir/src/module.rs`
- Codegen: `crates/agentus-codegen/src/compiler.rs`
- Runtime: `crates/agentus-runtime/src/vm.rs`

## Running Targeted Tests

```bash
# Run a single test
cargo test --workspace -- test_hello_world

# Run tests matching a pattern
cargo test --workspace -- test_agent

# Run only integration tests
cargo test -p agentus-codegen --test end_to_end

# Run only unit tests for a specific crate
cargo test -p agentus-lexer
cargo test -p agentus-parser
cargo test -p agentus-runtime

# Run with output visible (useful for debugging)
cargo test --workspace -- --nocapture test_name
```

## Interpreting Failures

### Compile-time failure (codegen returns Err)
```
thread 'test_name' panicked at 'compile error: undefined variable ...'
```
- Check sema (`resolver.rs`) — is the variable/function being defined?
- Check codegen (`compiler.rs`) — is the AST node being handled?

### Runtime failure (VM returns Err)
```
thread 'test_name' panicked at 'runtime error: ...'
```
- Check the VM (`vm.rs`) — is the opcode implemented?
- Check instruction encoding — are multi-instruction sequences correct?
- Add debug output: `eprintln!` in the VM's main loop to trace execution.

### Parse failure
```
thread 'test_name' panicked at 'parse error: unexpected token ...'
```
- Check if the token exists in `token.rs`
- Check if the parser handles the new syntax in `parser.rs`
- Verify the lexer produces the expected tokens

### Test assertion failure
```
assertion `left == right` failed
  left: ["42"]
 right: ["43"]
```
- The program compiled and ran but produced wrong output
- Usually a codegen or runtime bug — check instruction emission or VM execution

## Debugging Tips

1. **Dump instructions**: Add `eprintln!` in `FunctionEmitter::emit()` to see all emitted instructions
2. **Trace VM execution**: Add `eprintln!` at the top of the VM's main loop to see PC, opcode, registers
3. **Inspect Module**: The Module has a `Debug` impl — print it after compilation
4. **Minimal repro**: Reduce the failing `.ags` program to the smallest case that fails
5. **Check multi-instruction sequences**: Off-by-one errors in Call/TCall/IterNext/MethodCall sequences are common

## Branching & Commit Style

### Recommended Branch Naming
```
feature/<phase>-<feature>    e.g., feature/phase5-send-recv
fix/<description>            e.g., fix/parser-map-literal
refactor/<description>       e.g., refactor/vm-value-types
docs/<description>           e.g., docs/architecture-update
```

### Commit Messages
- Use imperative mood: "Add Send/Recv opcodes" not "Added Send/Recv opcodes"
- Reference the phase when relevant: "Phase 5: Implement agent message queues"
- Keep the first line under 72 characters
- For multi-part changes, use a body explaining the "why"

### Workflow
1. Create a feature branch from `main`
2. Make small, focused commits (one logical change per commit)
3. Run `make smoke` before pushing
4. Update `claude-progress.txt` with what was done
5. Update `spec/feature-checklist.md` for any features completed

## Code Style

- Run `cargo fmt --all` before committing
- Run `cargo clippy --workspace` and fix warnings
- Follow existing patterns in the codebase:
  - `thiserror` v2 for error types
  - `serde` derive for serializable types
  - Inline unit tests in `#[cfg(test)] mod tests`
  - Integration tests in `crates/agentus-codegen/tests/end_to_end.rs`
