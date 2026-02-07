# Agentus Feature Checklist

Status key: `[x]` = DONE (verified), `[ ]` = NOT DONE, `[~]` = PARTIAL

---

## Phase 1: Vertical Slice (Language Core)

### Literals
- [x] Number literals (`42`, `3.14`)
  - Verify: `cargo test --workspace -- test_number_literal`
- [x] String literals (`"hello"`)
  - Verify: `cargo test --workspace -- test_string_literal`
- [x] Boolean literals (`true`, `false`)
  - Verify: `cargo test --workspace -- test_bool_literals`
- [x] None literal
  - Verify: `cargo test --workspace -- test_none_literal`

### Variables
- [x] `let` bindings
  - Verify: `cargo test --workspace -- test_hello_world`
- [x] Multiple `let` statements
  - Verify: `cargo test --workspace -- test_multiple_lets`
- [x] Variable reassignment
  - Verify: `cargo test --workspace -- test_variable_reassignment`

### Output
- [x] `emit` statement
  - Verify: `cargo test --workspace -- test_hello_world`
- [x] Emit different types (num, str, bool, none)
  - Verify: `cargo test --workspace -- test_output_types`

### Arithmetic
- [x] Addition (`+`)
  - Verify: `cargo test --workspace -- test_addition`
- [x] Subtraction (`-`)
  - Verify: `cargo test --workspace -- test_subtraction`
- [x] Multiplication (`*`)
  - Verify: `cargo test --workspace -- test_multiplication`
- [x] Division (`/`)
  - Verify: `cargo test --workspace -- test_division`
- [x] Modulo (`%`)
  - Verify: `cargo test --workspace -- test_modulo`
- [x] Negation (unary `-`)
  - Verify: `cargo test --workspace -- test_negation`
- [x] Operator precedence
  - Verify: `cargo test --workspace -- test_operator_precedence`
- [x] Complex arithmetic expressions
  - Verify: `cargo test --workspace -- test_complex_arithmetic`

### Comparison
- [x] Greater than (`>`)
  - Verify: `cargo test --workspace -- test_greater_than`
- [x] Less than (`<`)
  - Verify: `cargo test --workspace -- test_less_than`
- [x] Equality (`==`)
  - Verify: `cargo test --workspace -- test_equality`
- [x] Inequality (`!=`)
  - Verify: `cargo test --workspace -- test_inequality`
- [x] Less/greater than or equal (`<=`, `>=`)
  - Verify: `cargo test --workspace -- test_lte_gte`

### Logic
- [x] And/Or (`and`, `or`)
  - Verify: `cargo test --workspace -- test_and_or`
- [x] Not (`not`)
  - Verify: `cargo test --workspace -- test_not`

### Strings
- [x] String concatenation (`++`)
  - Verify: `cargo test --workspace -- test_string_concat`

---

## Phase 2: Expressions & Control Flow

### Control Flow
- [x] `if` (true branch)
  - Verify: `cargo test --workspace -- test_if_true_branch`
- [x] `if` (false branch)
  - Verify: `cargo test --workspace -- test_if_false_branch`
- [x] `if/else`
  - Verify: `cargo test --workspace -- test_if_else`
- [x] `if` with arithmetic condition
  - Verify: `cargo test --workspace -- test_if_with_arithmetic`
- [x] `while` loop
  - Verify: `cargo test --workspace -- test_while_basic`
- [x] `while` never executes (false condition)
  - Verify: `cargo test --workspace -- test_while_never_executes`
- [x] `while` with accumulator
  - Verify: `cargo test --workspace -- test_while_sum`
- [x] `for` loop over list
  - Verify: `cargo test --workspace -- test_for_basic`
- [x] `for` loop over empty list
  - Verify: `cargo test --workspace -- test_for_empty_list`
- [x] `for` loop with numbers
  - Verify: `cargo test --workspace -- test_for_with_numbers`

### Functions
- [x] Function definition and call
  - Verify: `cargo test --workspace -- test_function_basic`
- [x] Two-parameter functions
  - Verify: `cargo test --workspace -- test_function_two_params`
- [x] Functions with variable args
  - Verify: `cargo test --workspace -- test_function_with_variable_args`
- [x] Multiple function calls
  - Verify: `cargo test --workspace -- test_function_multiple_calls`
- [x] Function returning string
  - Verify: `cargo test --workspace -- test_function_string_return`
- [x] Function with if/else
  - Verify: `cargo test --workspace -- test_function_with_if`

### String Interpolation
- [x] Simple variable interpolation (`"Hello, {name}"`)
  - Verify: `cargo test --workspace -- test_interpolation_simple`
- [x] Expression interpolation (`"{a + b}"`)
  - Verify: `cargo test --workspace -- test_interpolation_expression`
- [x] Interpolation with function calls
  - Verify: `cargo test --workspace -- test_interpolation_with_function_call`
- [x] Expression-only template
  - Verify: `cargo test --workspace -- test_interpolation_only_expr`
- [x] String without interpolation (plain)
  - Verify: `cargo test --workspace -- test_no_interpolation`

### Complex Scenarios
- [x] Fibonacci (recursive functions)
  - Verify: `cargo test --workspace -- test_fibonacci`
- [x] Countdown (while + emit)
  - Verify: `cargo test --workspace -- test_countdown`
- [x] Nested if in while
  - Verify: `cargo test --workspace -- test_nested_if_in_while`
- [x] For with if
  - Verify: `cargo test --workspace -- test_for_with_if`

### Error Detection
- [x] Undefined variable error
  - Verify: `cargo test --workspace -- test_undefined_variable_error`
- [x] Undefined function error
  - Verify: `cargo test --workspace -- test_undefined_function_error`

---

## Phase 3: Agent Core

### Agent Definition
- [x] Agent definition with model, system prompt, memory, methods
  - Verify: `cargo test --workspace -- test_agent_with_model_and_prompt`
- [x] Agent instantiation (`AgentName()` → Spawn)
  - Verify: `cargo test --workspace -- test_agent_instantiation`
- [x] Multiple agent instances
  - Verify: `cargo test --workspace -- test_multiple_agent_instances`

### Agent Memory
- [x] Memory field defaults (num)
  - Verify: `cargo test --workspace -- test_agent_memory_persistence`
- [x] Memory field defaults (string)
  - Verify: `cargo test --workspace -- test_agent_memory_default_string`
- [x] `self.field` read (MLoad)
  - Verify: `cargo test --workspace -- test_agent_memory_persistence`
- [x] `self.field` write (MStore)
  - Verify: `cargo test --workspace -- test_agent_memory_persistence`
- [x] Memory persistence across method calls
  - Verify: `cargo test --workspace -- test_agent_memory_persistence`

### Agent Methods
- [x] Method dispatch (3-instruction sentinel sequence)
  - Verify: `cargo test --workspace -- test_agent_method_simple_return`
- [x] Method with no params
  - Verify: `cargo test --workspace -- test_agent_method_no_params`
- [x] Full agent example (memory + methods + exec)
  - Verify: `cargo test --workspace -- test_agent_full_example`

### Exec Blocks
- [x] `exec { "prompt" }` block (LLM call via HostInterface)
  - Verify: `cargo test --workspace -- test_exec_block_echo`
- [x] Inline exec expression
  - Verify: `cargo test --workspace -- test_exec_block_inline`

### Host Interface
- [x] `HostInterface` trait (`exec`, `tool_call`)
  - Verify: inspect `crates/agentus-runtime/src/host.rs`
- [x] `EchoHost` (returns prompt as response)
  - Verify: `cargo test --workspace -- test_exec_block_echo`
- [x] `NoHost` (errors on any call)
  - Verify: inspect `crates/agentus-runtime/src/host.rs`

---

## Phase 4: Tools

### Tool Definition
- [x] Tool declaration with description, params, return type
  - Verify: `cargo test --workspace -- test_tool_basic_call`
- [x] Tool with no description
  - Verify: `cargo test --workspace -- test_tool_no_description`
- [x] Tool with default parameters
  - Verify: `cargo test --workspace -- test_tool_default_params`
- [x] Default parameter override
  - Verify: `cargo test --workspace -- test_tool_default_override`

### Tool Calls
- [x] Basic tool call
  - Verify: `cargo test --workspace -- test_tool_basic_call`
- [x] Two-parameter tool call
  - Verify: `cargo test --workspace -- test_tool_two_params`
- [x] Tool call with variable args
  - Verify: `cargo test --workspace -- test_tool_with_variable_args`
- [x] Multiple tool calls
  - Verify: `cargo test --workspace -- test_tool_multiple_calls`
- [x] Tool call result in variable
  - Verify: `cargo test --workspace -- test_tool_result_in_variable`
- [x] Tool call from agent method
  - Verify: `cargo test --workspace -- test_tool_with_agent`
- [x] Undefined tool error
  - Verify: `cargo test --workspace -- test_undefined_tool_error`

---

## Phase 5: Multi-Agent (IN PROGRESS)

### Agent Communication
- [x] `send` statement (Send opcode) — send message to another agent
  - Verify: `cargo test --workspace -- test_send_recv_basic`
- [x] `recv` expression (Recv opcode) — receive message from queue (non-blocking)
  - Verify: `cargo test --workspace -- test_recv_empty`
- [x] FIFO ordering preserved in mailbox
  - Verify: `cargo test --workspace -- test_send_recv_ordering`
- [x] Independent mailboxes per agent
  - Verify: `cargo test --workspace -- test_send_recv_multiple_agents`
- [x] Send/recv with different value types
  - Verify: `cargo test --workspace -- test_send_different_types`
- [ ] `recv` with timeout (RecvTimeout opcode)
  - Verify: TBD
- [ ] `wait` expression (Wait opcode) — wait for agent completion
  - Verify: TBD

### Agent Lifecycle
- [ ] `kill` statement (Kill opcode) — terminate an agent
  - Verify: TBD
- [ ] Agent status checking
  - Verify: TBD

### Cooperative Scheduling
- [ ] Yield at Exec points
  - Verify: TBD
- [ ] Yield at TCall points
  - Verify: TBD
- [ ] Yield at Recv/Wait points
  - Verify: TBD
- [ ] Scheduler implementation (round-robin or event-driven)
  - Verify: TBD

### Multi-Agent Examples
- [x] Basic message passing between agents
  - Verify: `cargo run -p agentus-cli -- exec examples/multi_agent.ags`
- [ ] Agent delegation pattern
  - Verify: TBD

---

## Phase 6: Collections (DONE)

### Map Operations
- [x] Map literal syntax (`{ "key": value, ... }`)
  - Verify: `cargo test --workspace -- test_map_literal_basic`
- [x] NewMap opcode in codegen
  - Verify: `cargo test --workspace -- test_map_literal_basic`
- [x] NewMap opcode in runtime
  - Verify: `cargo test --workspace -- test_map_literal_basic`
- [x] Map index get (`map["key"]`)
  - Verify: `cargo test --workspace -- test_map_literal_basic`
- [x] Map index set (`map["key"] = value`)
  - Verify: `cargo test --workspace -- test_map_index_set`
- [x] Map `len()` (built-in function)
  - Verify: `cargo test --workspace -- test_map_len`
- [x] Map `.len()` method
  - Verify: `cargo test --workspace -- test_map_method_len`
- [x] Map `.contains()` method
  - Verify: `cargo test --workspace -- test_map_method_contains`
- [x] Map `.remove()` method
  - Verify: `cargo test --workspace -- test_map_method_remove`
- [x] Map `.keys()` method
  - Verify: `cargo test --workspace -- test_map_method_keys`
- [x] Map `.values()` method
  - Verify: `cargo test --workspace -- test_map_method_values`
- [x] Map iteration (`for key in map`)
  - Verify: `cargo test --workspace -- test_map_for_iteration`

### List Enhancements
- [x] List `push()` method
  - Verify: `cargo test --workspace -- test_list_push`
- [x] List index set (`list[i] = value`)
  - Verify: `cargo test --workspace -- test_list_index_set`
- [x] List `.len()` method
  - Verify: `cargo test --workspace -- test_list_method_len`
- [x] `len()` built-in function (lists, maps, strings)
  - Verify: `cargo test --workspace -- test_list_len`

### Collection Examples
- [x] Collections example
  - Verify: `cargo run -p agentus-cli -- exec examples/collections.ags`

---

## Phase 7: Error Handling + Resilience + JSON (DONE)

### Try/Catch/Throw
- [x] `try` block (TryBegin/TryEnd opcodes)
  - Verify: `cargo test --workspace -- test_try_catch_basic`
- [x] `catch` block with error variable
  - Verify: `cargo test --workspace -- test_try_catch_basic`
- [x] `throw` statement (Throw opcode)
  - Verify: `cargo test --workspace -- test_throw_unhandled`
- [x] Error value access (GetError opcode)
  - Verify: `cargo test --workspace -- test_try_catch_basic`
- [x] Nested try/catch
  - Verify: `cargo test --workspace -- test_nested_try_catch`
- [x] Throw from inside functions (call stack unwinding)
  - Verify: `cargo test --workspace -- test_try_catch_in_function`
- [x] Try/catch normal (no error) path
  - Verify: `cargo test --workspace -- test_try_catch_no_error`

### Assert
- [x] `assert condition` (default message)
  - Verify: `cargo test --workspace -- test_assert_fail_default_message`
- [x] `assert condition, "message"` (custom message)
  - Verify: `cargo test --workspace -- test_assert_fail_custom_message`
- [x] Assert passing (no error)
  - Verify: `cargo test --workspace -- test_assert_pass`
- [x] Assert caught by try/catch
  - Verify: `cargo test --workspace -- test_assert_caught_by_try`
- [x] Assert with expressions
  - Verify: `cargo test --workspace -- test_assert_with_expression`

### Retry
- [x] `retry N { body }` expression
  - Verify: `cargo test --workspace -- test_retry_no_error`
- [x] Retry with error recovery
  - Verify: `cargo test --workspace -- test_retry_with_counter`
- [x] Retry exhausted (re-throws last error)
  - Verify: `cargo test --workspace -- test_retry_exhausted`
- [x] Retry with assert
  - Verify: `cargo test --workspace -- test_retry_with_assert`

### JSON Built-ins
- [x] `parse_json(str)` — parse JSON string into Value
  - Verify: `cargo test --workspace -- test_parse_json_object`
- [x] `to_json(value)` — serialize Value to JSON string
  - Verify: `cargo test --workspace -- test_to_json_list`
- [x] JSON parse error throws (catchable)
  - Verify: `cargo test --workspace -- test_parse_json_error_caught`
- [x] JSON roundtrip (to_json + parse_json)
  - Verify: `cargo test --workspace -- test_parse_json_roundtrip`
- [x] Nested JSON objects/arrays
  - Verify: `cargo test --workspace -- test_parse_json_nested`

### Error Handling Examples
- [x] Error handling example
  - Verify: `cargo run -p agentus-cli -- exec examples/error_handling.ags`

---

## Phase 8: Pipelines (TODO)

- [ ] `pipeline` definition syntax
  - Verify: TBD
- [ ] `stage` definition within pipeline
  - Verify: TBD
- [ ] PipelineRun opcode
  - Verify: TBD
- [ ] Pipeline data flow between stages
  - Verify: TBD

---

## Phase 9: Polish (TODO)

### Serialization
- [ ] Binary .agc format serialization (Module → bytes)
  - Verify: TBD
- [ ] Binary .agc format deserialization (bytes → Module)
  - Verify: TBD
- [ ] `compile` CLI command produces .agc file
  - Verify: TBD
- [ ] `exec` CLI command can load .agc file
  - Verify: TBD

### Error Messages
- [ ] Parser error recovery (report multiple errors)
  - Verify: TBD
- [ ] Source location in runtime errors
  - Verify: TBD
- [ ] Colored/formatted error output
  - Verify: TBD

### Language Server Protocol
- [ ] Basic LSP server
  - Verify: TBD
- [ ] Syntax highlighting support
  - Verify: TBD
- [ ] Go-to-definition
  - Verify: TBD
- [ ] Error diagnostics
  - Verify: TBD

---

## Non-Functional / Infrastructure

### Build & CI
- [x] Cargo workspace builds cleanly
  - Verify: `cargo build --workspace`
- [ ] CI pipeline (GitHub Actions)
  - Verify: check `.github/workflows/`
- [x] Makefile/justfile for common tasks
  - Verify: `make smoke`

### Code Quality
- [~] `cargo fmt` clean (has diffs, fixable with `cargo fmt --all`)
  - Verify: `cargo fmt --check --all`
- [~] `cargo clippy` clean (9 warnings, 0 errors)
  - Verify: `cargo clippy --workspace`
- [ ] No `unsafe` code
  - Verify: `grep -r "unsafe" crates/`

### Testing
- [x] Integration test framework (`run`, `run_with_host`, `run_values`, `expect_compile_error`)
  - Verify: `cargo test --workspace -- end_to_end`
- [ ] CLI integration tests
  - Verify: none exist yet
- [ ] Fuzzing / property-based tests
  - Verify: none exist yet

### Documentation
- [x] CLAUDE.md (project manual)
- [x] claude-progress.txt (status tracker)
- [x] spec/feature-checklist.md (this file)
- [x] docs/architecture.md
- [x] docs/dev-workflow.md

### Bytecode Stability
- [ ] Module format version field
  - Verify: inspect `Module` struct — no version field exists
- [ ] Instruction encoding documented and stable
  - Verify: `docs/architecture.md`
- [ ] Opcode numbering stable (no renumbering without migration)
  - Verify: inspect `opcode.rs` — values are manually assigned
