use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "exec" => {
            if args.len() < 3 {
                eprintln!("Usage: agentus exec <file.ags>");
                process::exit(1);
            }
            cmd_exec(&args[2]);
        }
        "compile" => {
            if args.len() < 3 {
                eprintln!("Usage: agentus compile <file.ags>");
                process::exit(1);
            }
            cmd_compile(&args[2]);
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        "version" | "--version" | "-V" => {
            println!("agentus {}", env!("CARGO_PKG_VERSION"));
        }
        other => {
            eprintln!("Unknown command: {}", other);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Agentus - Agent Orchestration Language");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  agentus exec <file.ags>      Compile and run a source file");
    eprintln!("  agentus compile <file.ags>   Compile a source file (output: .agc)");
    eprintln!("  agentus version              Show version");
    eprintln!("  agentus help                 Show this help");
}

/// Compile and execute a .ags source file.
fn cmd_exec(path: &str) {
    // Read source
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            process::exit(1);
        }
    };

    // Lex
    let (tokens, lex_errors) = agentus_lexer::lexer::Lexer::new(&source).tokenize();
    if !lex_errors.is_empty() {
        for err in &lex_errors {
            eprintln!("Lexer error: {}", err);
        }
        process::exit(1);
    }

    // Parse
    let program = match agentus_parser::parser::Parser::new(tokens).parse() {
        Ok(p) => p,
        Err(errors) => {
            for err in &errors {
                eprintln!("Parse error: {}", err);
            }
            process::exit(1);
        }
    };

    // Semantic analysis
    if let Err(errors) = agentus_sema::resolver::resolve(&program) {
        for err in &errors {
            eprintln!("Semantic error: {}", err);
        }
        process::exit(1);
    }

    // Compile to bytecode
    let module = match agentus_codegen::compiler::Compiler::new().compile(&program) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Codegen error: {}", e);
            process::exit(1);
        }
    };

    // Run
    let mut vm = agentus_runtime::vm::VM::new(module)
        .with_host(Box::new(agentus_runtime::host::EchoHost));
    if let Err(e) = vm.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

/// Compile a .ags source file to bytecode (placeholder).
fn cmd_compile(path: &str) {
    // For now, just verify compilation succeeds
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            process::exit(1);
        }
    };

    match agentus_codegen::compiler::compile(&source) {
        Ok(_module) => {
            let out_path = path.replace(".ags", ".agc");
            println!("Compiled successfully: {} -> {}", path, out_path);
            // TODO: serialize module to .agc binary format
        }
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    }
}
