# Agentus Architecture

## Overview

Agentus is a domain-specific language for AI agent orchestration. It compiles `.ags` source files to a bytecode IR, which is executed by a register-based virtual machine. The implementation is in Rust, organized as a Cargo workspace with 8 crates.

## Compilation Pipeline

```
Source (.ags)
    │
    ▼
┌──────────┐   Tokens    ┌──────────┐    AST     ┌──────────┐
│  Lexer   │ ──────────► │  Parser  │ ─────────► │   Sema   │
│ (lexer)  │             │ (parser) │            │  (sema)  │
└──────────┘             └──────────┘            └──────────┘
                                                      │
                                              Validated AST
                                                      │
                                                      ▼
┌──────────┐  Module     ┌──────────┐
│ Runtime  │ ◄────────── │ Codegen  │
│   (VM)   │             │(codegen) │
└──────────┘             └──────────┘
      │
      ▼
  HostInterface (LLM, tools)
```

## Crate Dependency Graph

```
agentus-common  (shared types: Span, errors)
    │
    ├── agentus-lexer     (tokenizer)
    │       │
    │       └── agentus-parser  (AST)
    │               │
    ├── agentus-ir  │         (bytecode IR)
    │       │       │
    │       └───┬───┘
    │           │
    │    agentus-sema         (name resolution)
    │           │
    │    agentus-codegen      (compiler: AST → Module)
    │           │
    │    agentus-runtime      (VM: executes Module)
    │           │
    └── agentus-cli           (entry point: uses all crates)
```

## Crate Responsibilities

### agentus-common
Shared types used across crates.
- `Span`: Source location (start/end byte offsets) for error reporting
- `errors`: Common error types

### agentus-lexer
Tokenizer with a state machine for string interpolation.
- Input: source string
- Output: `Vec<Token>` where each `Token` has a `TokenKind` and `Span`
- Key complexity: String interpolation uses a mode stack (`Normal`, `StringInterp { brace_depth }`) to handle `"text {expr} more"` syntax, tracking brace nesting inside interpolated expressions.
- Token kinds: keywords (`let`, `if`, `while`, `for`, `fn`, `return`, `emit`, `agent`, `tool`, `exec`, etc.), operators, literals, identifiers, punctuation.

### agentus-parser
Recursive descent parser with Pratt parsing for expressions.
- Input: `Vec<Token>`
- Output: `Program` (AST root containing `Vec<Stmt>`)
- Statements: `Let`, `Emit`, `Return`, `If`, `While`, `For`, `FnDef`, `AgentDef`, `ToolDef`, `Assign`, `FieldAssign`, `ExprStmt`
- Expressions: literals, identifiers, binary/unary ops, function calls, method calls, field access, index access, list/map literals, exec blocks, template literals (string interpolation)
- Pratt parsing handles operator precedence for binary expressions.

### agentus-sema
Minimal semantic analysis (currently name resolution only).
- Input: `&Program` (AST)
- Output: `Result<(), Vec<String>>` (list of errors)
- Tracks variable definitions in a scope stack. Validates that variables are defined before use. Registers function/agent/tool names in global scope. Handles `self` in agent methods.
- **Not yet implemented**: type checking, type inference, return type validation.

### agentus-ir
Bytecode intermediate representation.
- **Opcodes** (`opcode.rs`): 67 opcodes across 16 categories, manually assigned u8 values. Categories: Control, Load/Store/Move, Agent Memory, Arithmetic, Comparison, Logic, String, Collection, Control Flow, Function Call/Return, LLM Execution, Agent Operations, Tool Invocation, Pipeline, I/O, Error Handling, Coroutine, Iterator, Type Operations.
- **Instructions** (`instruction.rs`): 32-bit fixed-width encoding with four formats:
  - `ABC`: opcode(8) | A(8) | B(8) | C(8) — three register operands
  - `ABx`: opcode(8) | A(8) | Bx(16) — register + unsigned 16-bit
  - `AsBx`: opcode(8) | A(8) | sBx(16) — register + signed 16-bit
  - `sBx`: opcode(8) | sBx(24) — signed 24-bit (no register)
- **Module** (`module.rs`): The compiled output, containing:
  - `constants: Vec<Constant>` — string pool, numbers, bools, None
  - `functions: Vec<Function>` — compiled function bodies (instructions + metadata)
  - `agents: Vec<AgentDescriptor>` — agent type definitions (model, prompt, memory, methods)
  - `tools: Vec<ToolDescriptor>` — tool declarations (description, params with defaults)
  - `entry_function: u32` — index of the main/entry function
- **ModuleBuilder**: Builder pattern for constructing modules during compilation, with constant deduplication.

### agentus-codegen
Compiler that translates AST to bytecode Module.
- **Compiler**: Owns a `ModuleBuilder`. Iterates over top-level statements.
- **FunctionEmitter**: Compiles a single function/method body. Manages:
  - Local register allocation (`next_register: u8`, max 256)
  - Local variable → register mapping (`locals: HashMap<String, u8>`)
  - Instruction emission
  - Sub-emitter creation for nested function/method definitions
- **Key patterns**:
  - Sub-emitters must copy `function_table`, `agent_table`, `tool_table` from parent
  - Multi-arg calls: compile all args first, then copy to consecutive registers
  - Agent/tool definitions emit descriptors to the Module
  - Method bodies are compiled as regular functions, dispatched via sentinel

### agentus-runtime
Register-based virtual machine.
- **VM**: Executes a Module. Main loop fetches/decodes/executes instructions.
  - `call_stack: Vec<CallFrame>` — function call stack
  - `agents: HashMap<u64, AgentInstance>` — live agent instances
  - `outputs: Vec<Value>` — collected emit outputs (for testing)
  - `host: Box<dyn HostInterface>` — LLM/tool boundary
- **CallFrame**: Per-function state with `registers: Vec<Value>`, `pc`, `return_info`, `agent_id`
- **Value** (`value.rs`): Runtime value type — `None`, `Bool(bool)`, `Num(f64)`, `Str(Rc<String>)`, `List(Rc<RefCell<Vec<Value>>>)`, `AgentHandle(u64)`, `Iterator(...)`
- **HostInterface** (`host.rs`): Trait with `exec(ExecRequest) -> Result<String>` and `tool_call(ToolCallRequest) -> Result<String>`. Implementations: `EchoHost` (testing), `NoHost` (default).

### agentus-cli
Thin CLI wrapper.
- `exec <file>`: Read → Lex → Parse → Resolve → Compile → Run
- `compile <file>`: Same pipeline but no execution (serialization not yet implemented)
- Uses `EchoHost` by default for exec (no real LLM connection yet)

## Multi-Instruction Sequences

Several operations require multiple consecutive instructions:

| Operation | Instructions | Notes |
|-----------|-------------|-------|
| Function call | `Call(result, func_idx)` + `Nop(0, arg_start, num_args)` | 2-instruction |
| Tool call | `TCall(result, tool_idx)` + `Nop(0, arg_start, num_args)` | 2-instruction |
| Iterator next | `IterNext(var, jump_offset)` + `Nop(0, iter_reg, 0)` | 2-instruction |
| Method call | `Call(result, 0xFFFE)` + `Nop(0, arg_start, num_args)` + `Nop(0, method_name_idx)` | 3-instruction, sentinel |

The sentinel value `0xFFFE` in Call's Bx field distinguishes method dispatch from regular calls. The VM reads the extra Nop instructions to get argument layout and method name.

## Memory Model

Three tiers of memory:

1. **Registers** (local): 256 per call frame. Allocated linearly during compilation. Freed when frame pops.
2. **Agent Memory** (persistent): Per-agent-instance `HashMap<String, Value>`. Accessed via `MLoad`/`MStore` opcodes. Keyed by field name from `AgentDescriptor.memory_fields`.
3. **Global Memory** (shared): Not yet implemented. Planned via `GLoad`/`GStore` opcodes.

## Host Interface Boundary

The `HostInterface` trait is the sole boundary between the VM and external services:

```
VM ─── HostInterface ─── LLM providers, tool implementations, external APIs
```

- `exec(ExecRequest)`: Send a prompt to an LLM, get a response string
- `tool_call(ToolCallRequest)`: Invoke a tool with named arguments, get a response string

The language declares tools (name, description, params, return type) but does NOT implement them. The host provides all tool implementations. This keeps the VM pure and testable.

## Invariants to Preserve

1. **Instruction encoding is 32-bit fixed-width.** Do not change this without a migration plan.
2. **Opcode values are manually assigned.** Do not renumber existing opcodes. Add new ones in their category's range.
3. **Module format fields are append-only.** Adding new fields to Module/AgentDescriptor/ToolDescriptor is safe. Removing or reordering fields breaks serialization.
4. **Register allocation is linear.** Registers are allocated sequentially within a function. The VM trusts `num_registers` in Function.
5. **HostInterface is the only external boundary.** The VM must not make system calls, network requests, or file I/O directly.
6. **EchoHost is the canonical test host.** All agent/tool tests should use EchoHost for determinism.
7. **Constant pool deduplication.** ModuleBuilder deduplicates constants. Two identical string literals share one constant index.
