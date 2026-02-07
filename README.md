# Agentus

A domain-specific language for AI agent orchestration. Agentus compiles `.ags` source files to bytecode IR and executes them on a register-based virtual machine.

## Features

- **Agent definitions** with model configuration, system prompts, persistent memory, and methods
- **LLM execution** via `exec {}` blocks with pluggable host interface
- **Typed language** with `str`, `num`, `bool`, `list`, `map`, and `agent_handle` types
- **String interpolation**: `"Hello, {name}!"`
- **Control flow**: `if`/`else`, `while`, `for..in` loops
- **Functions** with typed parameters and return types
- **Register-based VM** with 256 registers per call frame and 67 opcodes
- **Three-tier memory**: registers (local), agent memory (persistent), global (shared)

## Quick Start

```bash
# Build
cargo build --workspace

# Run an example
cargo run -p agentus-cli -- exec examples/agent_basic.ags

# Run tests
cargo test --workspace
```

## Language Overview

### Hello World

```
let greeting = "Hello Agentus!"
emit greeting
```

### Variables and Expressions

```
let x = 42
let name = "World"
let is_big = x > 10
emit "x is {x}, big = {is_big}"
```

### Functions

```
fn add(a: num, b: num) -> num {
    return a + b
}

fn greet(name: str) -> str {
    return "Hello, " ++ name ++ "!"
}

emit add(10, 32)
emit greet("Agentus")
```

### Control Flow

```
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

let items = [1, 2, 3, 4, 5]
for item in items {
    emit item
}

let i = 0
while i < 5 {
    emit i
    i = i + 1
}
```

### Agents

Agents are stateful entities with persistent memory, methods, and optional LLM configuration:

```
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
emit c.increment()    // 1
emit c.increment()    // 2
emit c.increment()    // 3
emit c.get_count()    // 3
```

### Tools

Tools are declaration-only interfaces. The host provides implementations at runtime:

```
tool get_weather {
    description { "Get weather for a location" }
    param location: str
    param units: str = "celsius"
    returns str
}

let weather = get_weather("London")           // units defaults to "celsius"
let weather2 = get_weather("Tokyo", "fahrenheit")
emit weather
emit weather2
```

### LLM Execution

The `exec` block sends a prompt to the configured LLM host:

```
let result = exec { "What is 2+2?" }
emit result
```

## Architecture

Agentus is implemented as a Rust workspace with 8 crates:

```
source.ags
    |
    v
 [Lexer]        agentus-lexer     Tokenization, string interpolation
    |
    v
 [Parser]       agentus-parser    Recursive descent + Pratt parsing -> AST
    |
    v
 [Resolver]     agentus-sema      Name resolution, scope checking
    |
    v
 [Compiler]     agentus-codegen   AST -> bytecode instructions
    |
    v
 [VM]           agentus-runtime   Register-based execution engine
```

Supporting crates:
- `agentus-common` — shared types (spans)
- `agentus-ir` — bytecode IR (opcodes, instructions, module format)
- `agentus-cli` — command-line interface

### Host Interface

The VM communicates with the outside world (LLM providers, tools) through the `HostInterface` trait. This allows swapping implementations for testing (`EchoHost` returns the prompt as the response) or production (real API calls).

## Examples

The `examples/` directory contains sample programs:

| File | Description |
|------|-------------|
| `hello.ags` | Hello world |
| `arithmetic.ags` | Math operations and precedence |
| `control_flow.ags` | If/else and loops |
| `functions.ags` | Function definitions and calls |
| `for_loop.ags` | For loop iteration |
| `while_loop.ags` | While loop |
| `interpolation.ags` | String interpolation |
| `agent_basic.ags` | Agent with memory and methods |
| `tools.ags` | Tool declarations and calls |

Run any example:

```bash
cargo run -p agentus-cli -- exec examples/<name>.ags
```

## Roadmap

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | Done | Vertical slice (let, emit, literals, basic ops) |
| 2 | Done | Expressions & control flow (if/else, while, for, functions, interpolation) |
| 3 | Done | Agent core (agent defs, exec blocks, agent memory, method dispatch) |
| 4 | Done | Tools (tool declarations, default params, host-provided tool implementations) |
| 5 | Planned | Multi-agent (send/recv/wait, cooperative async scheduling) |
| 6 | Planned | Collections (map literals, map operations) |
| 7 | Planned | Error handling (try/catch/throw) |
| 8 | Planned | Pipelines (pipeline/stage syntax) |
| 9 | Planned | Polish (binary serialization, error messages, LSP) |

## License

MIT
