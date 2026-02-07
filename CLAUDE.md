# Agentus

Domain-specific language for AI agent orchestration. Compiles `.ags` source to bytecode IR, executed by a register-based VM. Implemented in Rust.

## Quick Reference

```bash
cargo test --workspace          # run all tests (104 currently)
cargo run -p agentus-cli -- exec examples/agent_basic.ags  # run an example
cargo build --workspace         # build everything
```

## Architecture

8-crate Cargo workspace. Pipeline: **source -> lexer -> parser -> sema -> codegen -> runtime**

| Crate | Purpose | Key files |
|-------|---------|-----------|
| `agentus-common` | Shared types (Span) | `src/span.rs` |
| `agentus-lexer` | Tokenizer with string interpolation state machine | `src/lexer.rs`, `src/token.rs` |
| `agentus-parser` | Recursive descent + Pratt parsing | `src/parser.rs`, `src/ast.rs` |
| `agentus-sema` | Name resolution, scope checking | `src/resolver.rs` |
| `agentus-ir` | Bytecode IR: opcodes, instructions, module | `src/opcode.rs`, `src/instruction.rs`, `src/module.rs` |
| `agentus-codegen` | AST -> bytecode compiler | `src/compiler.rs` |
| `agentus-runtime` | Register-based VM, host interface | `src/vm.rs`, `src/host.rs`, `src/value.rs` |
| `agentus-cli` | CLI entry point | `src/main.rs` |

## VM Design

- Register-based, 256 registers per call frame
- 32-bit fixed-width instructions (ABC, ABx, AsBx formats)
- 67 opcodes across 16 categories
- Three-tier memory: registers (local), agent memory (persistent), global (shared)
- `HostInterface` trait = boundary between VM and outside world (LLM, tools)

## Instruction Sequences

- **Regular function calls**: 2-instruction sequence — `Call(result_reg, func_idx)` + `Nop(0, first_arg_reg, num_args)`
- **Iterator advance**: 2-instruction sequence — `IterNext(var_reg, jump_offset)` + `Nop(0, iter_reg, 0)`
- **Method calls**: 3-instruction sequence — `Call(result_reg, 0xFFFE)` + `Nop(0, first_arg_reg, num_args_with_handle)` + `Nop(0, method_name_const_idx)`

The sentinel value `0xFFFE` in the Call instruction's Bx field signals method dispatch rather than a regular function call.

## Key Patterns

- **FunctionEmitter sub-emitters**: When compiling function/method bodies, create a new `FunctionEmitter` with `self.builder`. Extract `instructions` and `next_register` from it before touching `self.builder` again (borrow checker).
- **Multi-arg calls**: Compile ALL arguments first to get their registers, THEN copy to consecutive registers. Don't interleave compile+alloc.
- **Agent method frames**: `CallFrame.agent_id` is set when entering a method. `MLoad`/`MStore` use this to find the right `AgentInstance.memory`.
- **Module.agents**: The `Module` struct has an `agents: Vec<AgentDescriptor>` field. Any test that creates a `Module` manually must include `agents: Vec::new()`.

## Testing

- Integration tests: `crates/agentus-codegen/tests/end_to_end.rs`
  - `run(source)` — compile + run, returns `Vec<String>` of emit outputs
  - `run_with_host(source, host)` — same but with a custom `HostInterface`
  - `run_values(source)` — returns raw `Vec<Value>`
  - `expect_compile_error(source, expected)` — asserts compilation fails
- Unit tests are inline in each crate
- Agent tests use `EchoHost` which returns the user prompt as the LLM response

## Implementation Status

- **Phase 1**: Vertical slice (let, emit, literals, basic ops)
- **Phase 2**: Expressions & control flow (if/else, while, for, functions, string interpolation)
- **Phase 3**: Agent core (agent defs, exec blocks, agent memory, method dispatch, HostInterface)
- **Phase 4** (TODO): Tools (tool declarations, TCall opcode, host-provided implementations)
- **Phase 5** (TODO): Multi-agent (Send/Recv/Wait, cooperative async scheduling)
- **Phase 6** (TODO): Collections (map literals, map operations)
- **Phase 7** (TODO): Error handling (try/catch/throw)
- **Phase 8** (TODO): Pipelines (pipeline/stage syntax, PipelineRun opcode)
- **Phase 9** (TODO): Polish (binary serialization for .agc, better error messages, LSP)

## File Extensions

- `.ags` — source code
- `.agc` — compiled bytecode (serialization not yet implemented)
