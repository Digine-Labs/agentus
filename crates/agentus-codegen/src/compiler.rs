use std::collections::HashMap;
use agentus_ir::instruction::Instruction;
use agentus_ir::module::{AgentDescriptor, AgentMemoryField, Function, ModuleBuilder, ToolDescriptor, ToolParamDescriptor};
use agentus_ir::opcode::OpCode;
use agentus_parser::ast::*;

/// Compiles an AST Program into a bytecode Module.
pub struct Compiler {
    builder: ModuleBuilder,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            builder: ModuleBuilder::new(),
        }
    }

    /// Compile a program into a Module.
    pub fn compile(mut self, program: &Program) -> Result<agentus_ir::module::Module, String> {
        let mut emitter = FunctionEmitter::new(&mut self.builder);

        for stmt in &program.statements {
            emitter.compile_stmt(stmt)?;
        }

        emitter.emit(Instruction::op_only(OpCode::Halt));

        let instructions = emitter.instructions;
        let num_registers = emitter.next_register;
        let locals = emitter.locals; // keep the compiler happy
        drop(locals);

        let func = Function {
            name_idx: self.builder.add_string_constant("__main__") as u32,
            num_params: 0,
            num_registers,
            instructions,
        };

        let entry = self.builder.add_function(func);
        self.builder.set_entry_function(entry);

        Ok(self.builder.build())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Emits bytecode instructions for a single function body.
struct FunctionEmitter<'a> {
    builder: &'a mut ModuleBuilder,
    instructions: Vec<Instruction>,
    /// Maps local variable names to register indices.
    locals: HashMap<String, u8>,
    /// Next available register.
    next_register: u8,
    /// Stack of function compilers for nested functions (Phase 2+).
    /// For Phase 1, we only compile the top-level script.
    function_table: Vec<(String, u32)>,
    /// Agent name → descriptor index in the module.
    agent_table: Vec<(String, u32)>,
    /// Tool name → (descriptor index, param defaults).
    tool_table: Vec<(String, u32, Vec<Option<u16>>)>,
}

impl<'a> FunctionEmitter<'a> {
    fn new(builder: &'a mut ModuleBuilder) -> Self {
        Self {
            builder,
            instructions: Vec::new(),
            locals: HashMap::new(),
            next_register: 0,
            function_table: Vec::new(),
            agent_table: Vec::new(),
            tool_table: Vec::new(),
        }
    }

    fn alloc_register(&mut self) -> u8 {
        let reg = self.next_register;
        assert!(reg < 255, "register overflow: too many local variables");
        self.next_register += 1;
        reg
    }

    fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    fn current_offset(&self) -> usize {
        self.instructions.len()
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let(l) => {
                let reg = self.compile_expr(&l.value)?;
                self.locals.insert(l.name.clone(), reg);
                Ok(())
            }
            Stmt::Emit(e) => {
                let reg = self.compile_expr(&e.value)?;
                self.emit(Instruction::op_a(OpCode::Emit, reg));
                Ok(())
            }
            Stmt::Return(r) => {
                if let Some(value) = &r.value {
                    let reg = self.compile_expr(value)?;
                    self.emit(Instruction::op_a(OpCode::Ret, reg));
                } else {
                    self.emit(Instruction::op_only(OpCode::RetNone));
                }
                Ok(())
            }
            Stmt::ExprStmt(e) => {
                self.compile_expr(e)?;
                Ok(())
            }
            Stmt::Assign(a) => {
                let reg = self.compile_expr(&a.value)?;
                if let Some(&existing) = self.locals.get(&a.name) {
                    self.emit(Instruction::abc(OpCode::Move, existing, reg, 0));
                } else {
                    return Err(format!("undefined variable '{}' in assignment", a.name));
                }
                Ok(())
            }
            Stmt::If(i) => self.compile_if(i),
            Stmt::While(w) => self.compile_while(w),
            Stmt::For(f) => self.compile_for(f),
            Stmt::FnDef(f) => self.compile_fn_def(f),
            Stmt::AgentDef(a) => self.compile_agent_def(a),
            Stmt::ToolDef(t) => self.compile_tool_def(t),
            Stmt::Send(s) => {
                let target_reg = self.compile_expr(&s.target)?;
                let msg_reg = self.compile_expr(&s.message)?;
                self.emit(Instruction::abc(OpCode::Send, target_reg, msg_reg, 0));
                Ok(())
            }
            Stmt::FieldAssign(fa) => {
                let val_reg = self.compile_expr(&fa.value)?;
                // Only self.field = expr is supported
                match &fa.object {
                    Expr::Ident(name, _) if name == "self" => {
                        let field_idx = self.builder.add_string_constant(&fa.field);
                        self.emit(Instruction::abx(OpCode::MStore, val_reg, field_idx));
                        Ok(())
                    }
                    _ => Err("field assignment is only supported on 'self'".to_string()),
                }
            }
        }
    }

    fn compile_if(&mut self, stmt: &IfStmt) -> Result<(), String> {
        let cond_reg = self.compile_expr(&stmt.condition)?;

        // JmpFalse cond_reg, offset (to else/end)
        let jump_to_else = self.current_offset();
        self.emit(Instruction::asbx(OpCode::JmpFalse, cond_reg, 0)); // placeholder

        // Then body
        for s in &stmt.then_body {
            self.compile_stmt(s)?;
        }

        if let Some(else_body) = &stmt.else_body {
            // Jump over else body
            let jump_over_else = self.current_offset();
            self.emit(Instruction::sbx(OpCode::Jmp, 0)); // placeholder

            // Patch jump-to-else
            let else_start = self.current_offset();
            let offset = (else_start as i16) - (jump_to_else as i16) - 1;
            self.instructions[jump_to_else] =
                Instruction::asbx(OpCode::JmpFalse, cond_reg, offset);

            // Else body
            for s in else_body {
                self.compile_stmt(s)?;
            }

            // Patch jump-over-else
            let after_else = self.current_offset();
            let offset = (after_else as i32) - (jump_over_else as i32) - 1;
            self.instructions[jump_over_else] = Instruction::sbx(OpCode::Jmp, offset);
        } else {
            // Patch jump-to-else to point to after the then body
            let after_then = self.current_offset();
            let offset = (after_then as i16) - (jump_to_else as i16) - 1;
            self.instructions[jump_to_else] =
                Instruction::asbx(OpCode::JmpFalse, cond_reg, offset);
        }

        Ok(())
    }

    fn compile_while(&mut self, stmt: &WhileStmt) -> Result<(), String> {
        let loop_start = self.current_offset();
        let cond_reg = self.compile_expr(&stmt.condition)?;

        // JmpFalse to after loop
        let jump_exit = self.current_offset();
        self.emit(Instruction::asbx(OpCode::JmpFalse, cond_reg, 0)); // placeholder

        for s in &stmt.body {
            self.compile_stmt(s)?;
        }

        // Jump back to loop start
        let jump_back = self.current_offset();
        let offset = (loop_start as i32) - (jump_back as i32) - 1;
        self.emit(Instruction::sbx(OpCode::Jmp, offset));

        // Patch exit jump
        let after_loop = self.current_offset();
        let exit_offset = (after_loop as i16) - (jump_exit as i16) - 1;
        self.instructions[jump_exit] =
            Instruction::asbx(OpCode::JmpFalse, cond_reg, exit_offset);

        Ok(())
    }

    fn compile_for(&mut self, stmt: &ForStmt) -> Result<(), String> {
        // Compile iterable
        let iter_source = self.compile_expr(&stmt.iterable)?;

        // Create iterator
        let iter_reg = self.alloc_register();
        self.emit(Instruction::abc(OpCode::IterInit, iter_reg, iter_source, 0));

        // Loop variable register
        let var_reg = self.alloc_register();
        self.locals.insert(stmt.variable.clone(), var_reg);

        let loop_start = self.current_offset();

        // Two-instruction IterNext sequence:
        // 1. IterNext A=var_reg, sBx=jump_offset_if_exhausted (placeholder)
        // 2. Extra data: B=iter_reg
        let iter_next_pos = self.current_offset();
        self.emit(Instruction::asbx(OpCode::IterNext, var_reg, 0)); // placeholder
        self.emit(Instruction::abc(OpCode::Nop, 0, iter_reg, 0)); // extra data

        // Body
        for s in &stmt.body {
            self.compile_stmt(s)?;
        }

        // Jump back to IterNext
        let jump_back = self.current_offset();
        let back_offset = (loop_start as i32) - (jump_back as i32) - 1;
        self.emit(Instruction::sbx(OpCode::Jmp, back_offset));

        // Patch IterNext exit offset
        // The jump offset is relative to PC AFTER the extra data word (PC = iter_next_pos + 2)
        let after_loop = self.current_offset();
        let exit_offset = (after_loop as i16) - (iter_next_pos as i16) - 2;
        self.instructions[iter_next_pos] =
            Instruction::asbx(OpCode::IterNext, var_reg, exit_offset);

        Ok(())
    }

    fn compile_fn_def(&mut self, func: &FnDef) -> Result<(), String> {
        // For Phase 1, we compile functions inline (not as separate function entries).
        // A proper implementation would create a separate Function in the module
        // and use the Call opcode. For now, we just define the function name.
        // TODO: Implement proper function compilation in Phase 2.

        // Compile function body in a separate emitter
        let (fn_instructions, fn_num_registers) = {
            let mut fn_emitter = FunctionEmitter::new(self.builder);
            // Propagate tables so functions can call tools, other functions, and agents
            fn_emitter.function_table = self.function_table.clone();
            fn_emitter.agent_table = self.agent_table.clone();
            fn_emitter.tool_table = self.tool_table.clone();
            for param in &func.params {
                let reg = fn_emitter.alloc_register();
                fn_emitter.locals.insert(param.name.clone(), reg);
            }
            for stmt in &func.body {
                fn_emitter.compile_stmt(stmt)?;
            }
            fn_emitter.emit(Instruction::op_only(OpCode::RetNone));
            (fn_emitter.instructions, fn_emitter.next_register)
        };

        let compiled_func = Function {
            name_idx: self.builder.add_string_constant(&func.name) as u32,
            num_params: func.params.len() as u8,
            num_registers: fn_num_registers,
            instructions: fn_instructions,
        };

        let func_idx = self.builder.add_function(compiled_func);
        self.function_table.push((func.name.clone(), func_idx));
        self.locals.insert(func.name.clone(), 0); // Register the name

        Ok(())
    }

    fn compile_agent_def(&mut self, agent: &AgentDef) -> Result<(), String> {
        // Add model/system_prompt to constant pool
        let model_idx = agent.model.as_ref().map(|m| self.builder.add_string_constant(m));
        let system_prompt_idx = agent.system_prompt.as_ref().map(|s| self.builder.add_string_constant(s));

        // Build memory field descriptors
        let mut memory_fields = Vec::new();
        for field in &agent.memory_fields {
            let name_idx = self.builder.add_string_constant(&field.name);
            let default_idx = field.default.as_ref().map(|expr| {
                match expr {
                    Expr::NumberLit(n, _) => self.builder.add_num_constant(*n),
                    Expr::StringLit(s, _) => self.builder.add_string_constant(s),
                    Expr::BoolLit(b, _) => self.builder.add_bool_constant(*b),
                    _ => self.builder.add_none_constant(),
                }
            });
            memory_fields.push(AgentMemoryField { name_idx, default_idx });
        }

        // Compile each method as a separate Function
        let mut methods = Vec::new();
        for method in &agent.methods {
            let method_name_idx = self.builder.add_string_constant(&method.name);

            let (fn_instructions, fn_num_registers) = {
                let mut fn_emitter = FunctionEmitter::new(self.builder);
                // Propagate tables so methods can call tools, functions, and agents
                fn_emitter.function_table = self.function_table.clone();
                fn_emitter.agent_table = self.agent_table.clone();
                fn_emitter.tool_table = self.tool_table.clone();
                // Methods don't get an implicit `self` register;
                // self.field is compiled as MLoad/MStore using the frame's agent_id
                for param in &method.params {
                    let reg = fn_emitter.alloc_register();
                    fn_emitter.locals.insert(param.name.clone(), reg);
                }
                for stmt in &method.body {
                    fn_emitter.compile_stmt(stmt)?;
                }
                fn_emitter.emit(Instruction::op_only(OpCode::RetNone));
                (fn_emitter.instructions, fn_emitter.next_register)
            };

            let compiled_func = Function {
                name_idx: self.builder.add_string_constant(&method.name) as u32,
                num_params: method.params.len() as u8,
                num_registers: fn_num_registers,
                instructions: fn_instructions,
            };

            let func_idx = self.builder.add_function(compiled_func);
            methods.push((method_name_idx, func_idx));
        }

        let name_idx = self.builder.add_string_constant(&agent.name);
        let descriptor = AgentDescriptor {
            name_idx,
            model_idx,
            system_prompt_idx,
            memory_fields,
            methods,
        };
        let desc_idx = self.builder.add_agent(descriptor);
        self.agent_table.push((agent.name.clone(), desc_idx));
        self.locals.insert(agent.name.clone(), 0); // register the name for resolution

        Ok(())
    }

    fn compile_tool_def(&mut self, tool: &ToolDef) -> Result<(), String> {
        let name_idx = self.builder.add_string_constant(&tool.name);
        let description_idx = tool.description.as_ref().map(|d| self.builder.add_string_constant(d));

        let mut params = Vec::new();
        let mut param_defaults = Vec::new();
        for param in &tool.params {
            let param_name_idx = self.builder.add_string_constant(&param.name);
            let default_idx = param.default.as_ref().map(|expr| {
                match expr {
                    Expr::NumberLit(n, _) => self.builder.add_num_constant(*n),
                    Expr::StringLit(s, _) => self.builder.add_string_constant(s),
                    Expr::BoolLit(b, _) => self.builder.add_bool_constant(*b),
                    _ => self.builder.add_none_constant(),
                }
            });
            params.push(ToolParamDescriptor {
                name_idx: param_name_idx,
                default_idx,
            });
            param_defaults.push(default_idx);
        }

        let descriptor = ToolDescriptor {
            name_idx,
            description_idx,
            params,
        };
        let desc_idx = self.builder.add_tool(descriptor);
        self.tool_table.push((tool.name.clone(), desc_idx, param_defaults));
        self.locals.insert(tool.name.clone(), 0); // register the name for resolution

        Ok(())
    }

    /// Compile an expression and return the register it's stored in.
    fn compile_expr(&mut self, expr: &Expr) -> Result<u8, String> {
        match expr {
            Expr::StringLit(s, _) => {
                let reg = self.alloc_register();
                let idx = self.builder.add_string_constant(s);
                self.emit(Instruction::abx(OpCode::LoadConst, reg, idx));
                Ok(reg)
            }
            Expr::TemplateLit(segments, _) => {
                // Compile each segment and chain with Concat
                let mut result_reg: Option<u8> = None;

                for segment in segments {
                    let seg_reg = match segment {
                        TemplateSegment::Literal(s) => {
                            let reg = self.alloc_register();
                            let idx = self.builder.add_string_constant(s);
                            self.emit(Instruction::abx(OpCode::LoadConst, reg, idx));
                            reg
                        }
                        TemplateSegment::Expr(expr) => {
                            let expr_reg = self.compile_expr(expr)?;
                            // Convert to string via Concat with empty string
                            // (Concat already converts both operands to strings)
                            expr_reg
                        }
                    };

                    result_reg = Some(match result_reg {
                        None => seg_reg,
                        Some(prev_reg) => {
                            let concat_reg = self.alloc_register();
                            self.emit(Instruction::abc(
                                OpCode::Concat,
                                concat_reg,
                                prev_reg,
                                seg_reg,
                            ));
                            concat_reg
                        }
                    });
                }

                Ok(result_reg.unwrap_or_else(|| {
                    // Empty template — return empty string
                    let reg = self.alloc_register();
                    let idx = self.builder.add_string_constant("");
                    self.emit(Instruction::abx(OpCode::LoadConst, reg, idx));
                    reg
                }))
            }
            Expr::NumberLit(n, _) => {
                let reg = self.alloc_register();
                let idx = self.builder.add_num_constant(*n);
                self.emit(Instruction::abx(OpCode::LoadConst, reg, idx));
                Ok(reg)
            }
            Expr::BoolLit(b, _) => {
                let reg = self.alloc_register();
                if *b {
                    self.emit(Instruction::op_a(OpCode::LoadTrue, reg));
                } else {
                    self.emit(Instruction::op_a(OpCode::LoadFalse, reg));
                }
                Ok(reg)
            }
            Expr::NoneLit(_) => {
                let reg = self.alloc_register();
                self.emit(Instruction::op_a(OpCode::LoadNone, reg));
                Ok(reg)
            }
            Expr::Ident(name, _) => {
                if let Some(&reg) = self.locals.get(name) {
                    Ok(reg)
                } else {
                    Err(format!("undefined variable '{}'", name))
                }
            }
            Expr::BinOp(left, op, right, _) => {
                let left_reg = self.compile_expr(left)?;
                let right_reg = self.compile_expr(right)?;
                let result_reg = self.alloc_register();
                let opcode = match op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                    BinOp::Mod => OpCode::Mod,
                    BinOp::Concat => OpCode::Concat,
                    BinOp::Eq => OpCode::Eq,
                    BinOp::Neq => OpCode::Neq,
                    BinOp::Lt => OpCode::Lt,
                    BinOp::Lte => OpCode::Lte,
                    BinOp::Gt => OpCode::Gt,
                    BinOp::Gte => OpCode::Gte,
                    BinOp::And => OpCode::And,
                    BinOp::Or => OpCode::Or,
                };
                self.emit(Instruction::abc(opcode, result_reg, left_reg, right_reg));
                Ok(result_reg)
            }
            Expr::UnaryOp(op, expr, _) => {
                let expr_reg = self.compile_expr(expr)?;
                let result_reg = self.alloc_register();
                let opcode = match op {
                    UnaryOp::Neg => OpCode::Neg,
                    UnaryOp::Not => OpCode::Not,
                };
                self.emit(Instruction::abc(opcode, result_reg, expr_reg, 0));
                Ok(result_reg)
            }
            Expr::FnCall(name, args, _) => {
                // Check agent_table first (agent instantiation)
                let agent_idx = self
                    .agent_table
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, idx)| *idx);

                if let Some(desc_idx) = agent_idx {
                    let result_reg = self.alloc_register();
                    self.emit(Instruction::abx(OpCode::Spawn, result_reg, desc_idx as u16));
                    return Ok(result_reg);
                }

                // Check tool_table next (tool invocation)
                let tool_info = self
                    .tool_table
                    .iter()
                    .find(|(n, _, _)| n == name)
                    .map(|(_, idx, defaults)| (*idx, defaults.clone()));

                if let Some((tool_desc_idx, param_defaults)) = tool_info {
                    // Compile explicit arguments
                    let mut arg_regs = Vec::new();
                    for arg in args {
                        arg_regs.push(self.compile_expr(arg)?);
                    }

                    // Fill in defaults for missing arguments
                    let total_params = param_defaults.len();
                    for i in args.len()..total_params {
                        if let Some(default_idx) = param_defaults[i] {
                            let reg = self.alloc_register();
                            self.emit(Instruction::abx(OpCode::LoadConst, reg, default_idx));
                            arg_regs.push(reg);
                        }
                    }

                    // Copy into consecutive destination registers
                    let first_arg_reg = self.next_register;
                    for &src_reg in &arg_regs {
                        let dest = self.alloc_register();
                        if src_reg != dest {
                            self.emit(Instruction::abc(OpCode::Move, dest, src_reg, 0));
                        }
                    }

                    let result_reg = self.alloc_register();
                    // Two-instruction TCall sequence:
                    // 1. TCall A=result_reg, Bx=tool_desc_idx
                    // 2. Nop A=0, B=first_arg_reg, C=num_args
                    self.emit(Instruction::abx(
                        OpCode::TCall,
                        result_reg,
                        tool_desc_idx as u16,
                    ));
                    self.emit(Instruction::abc(
                        OpCode::Nop,
                        0,
                        first_arg_reg,
                        arg_regs.len() as u8,
                    ));
                    return Ok(result_reg);
                }

                // Find the function index
                let func_idx = self
                    .function_table
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, idx)| *idx);

                if let Some(func_idx) = func_idx {
                    // Compile all arguments first (may allocate non-consecutive registers)
                    let mut arg_regs = Vec::new();
                    for arg in args {
                        arg_regs.push(self.compile_expr(arg)?);
                    }

                    // Now copy into consecutive destination registers
                    let first_arg_reg = self.next_register;
                    for &src_reg in &arg_regs {
                        let dest = self.alloc_register();
                        if src_reg != dest {
                            self.emit(Instruction::abc(OpCode::Move, dest, src_reg, 0));
                        }
                    }

                    let result_reg = self.alloc_register();
                    // Two-instruction call sequence:
                    // 1. Call A=result_reg, Bx=func_idx
                    // 2. Extra data word: B=first_arg_reg, C=num_args
                    self.emit(Instruction::abx(
                        OpCode::Call,
                        result_reg,
                        func_idx as u16,
                    ));
                    self.emit(Instruction::abc(
                        OpCode::Nop, // extra data word (opcode ignored by VM)
                        0,
                        first_arg_reg,
                        args.len() as u8,
                    ));
                    Ok(result_reg)
                } else {
                    Err(format!("undefined function or tool '{}'", name))
                }
            }
            Expr::MethodCall(obj, method_name, args, _) => {
                // Compile receiver
                let obj_reg = self.compile_expr(obj)?;

                // Compile all args
                let mut arg_regs = Vec::new();
                for arg in args {
                    arg_regs.push(self.compile_expr(arg)?);
                }

                // Copy handle + args to consecutive registers
                let first_arg_reg = self.next_register;

                // First: the handle
                let handle_dest = self.alloc_register();
                if obj_reg != handle_dest {
                    self.emit(Instruction::abc(OpCode::Move, handle_dest, obj_reg, 0));
                }

                // Then: the arguments
                for &src_reg in &arg_regs {
                    let dest = self.alloc_register();
                    if src_reg != dest {
                        self.emit(Instruction::abc(OpCode::Move, dest, src_reg, 0));
                    }
                }

                let num_args_with_handle = (1 + args.len()) as u8;
                let method_name_idx = self.builder.add_string_constant(method_name);
                let result_reg = self.alloc_register();

                // Three-instruction method call sequence:
                // 1. Call A=result_reg, Bx=0xFFFE (sentinel)
                // 2. Nop A=0, B=first_arg_reg, C=num_args_with_handle
                // 3. Nop A=0, Bx=method_name_const_idx
                self.emit(Instruction::abx(OpCode::Call, result_reg, 0xFFFE));
                self.emit(Instruction::abc(OpCode::Nop, 0, first_arg_reg, num_args_with_handle));
                self.emit(Instruction::abx(OpCode::Nop, 0, method_name_idx));

                Ok(result_reg)
            }
            Expr::FieldAccess(obj, field, _) => {
                // self.field -> MLoad
                match obj.as_ref() {
                    Expr::Ident(name, _) if name == "self" => {
                        let field_idx = self.builder.add_string_constant(field);
                        let result_reg = self.alloc_register();
                        self.emit(Instruction::abx(OpCode::MLoad, result_reg, field_idx));
                        Ok(result_reg)
                    }
                    _ => Err("field access is only supported on 'self'".to_string()),
                }
            }
            Expr::IndexAccess(obj, index, _) => {
                let obj_reg = self.compile_expr(obj)?;
                let idx_reg = self.compile_expr(index)?;
                let result_reg = self.alloc_register();
                self.emit(Instruction::abc(OpCode::IndexGet, result_reg, obj_reg, idx_reg));
                Ok(result_reg)
            }
            Expr::ListLit(elems, _) => {
                let first_reg = self.next_register;
                for elem in elems {
                    self.compile_expr(elem)?;
                }
                let result_reg = self.alloc_register();
                self.emit(Instruction::abc(
                    OpCode::NewList,
                    result_reg,
                    first_reg,
                    elems.len() as u8,
                ));
                Ok(result_reg)
            }
            Expr::MapLit(_, _) => {
                Err("map literals not yet implemented".to_string())
            }
            Expr::ExecBlock(prompt, _) => {
                let prompt_reg = self.compile_expr(prompt)?;
                let result_reg = self.alloc_register();
                self.emit(Instruction::abc(OpCode::Exec, result_reg, prompt_reg, 0));
                Ok(result_reg)
            }
            Expr::Recv(target, _) => {
                let target_reg = self.compile_expr(target)?;
                let result_reg = self.alloc_register();
                self.emit(Instruction::abc(OpCode::Recv, result_reg, target_reg, 0));
                Ok(result_reg)
            }
        }
    }
}

/// Convenience: compile source code directly to a Module.
pub fn compile(source: &str) -> Result<agentus_ir::module::Module, String> {
    let program = agentus_parser::parser::parse(source).map_err(|errs| errs.join("; "))?;
    agentus_sema::resolver::resolve(&program).map_err(|errs| errs.join("; "))?;
    Compiler::new().compile(&program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentus_ir::opcode::OpCode;

    #[test]
    fn test_compile_let_emit() {
        let module = compile("let x = \"hello\"\nemit x").unwrap();

        assert_eq!(module.constants.len(), 2); // "hello" and "__main__"
        assert_eq!(module.functions.len(), 1);

        let func = &module.functions[0];
        assert!(func.instructions.len() >= 3); // LoadConst, Emit, Halt

        // First instruction: LoadConst r0, K0
        let inst0 = &func.instructions[0];
        assert_eq!(inst0.opcode(), Some(OpCode::LoadConst));
        assert_eq!(inst0.a(), 0); // r0

        // Second instruction: Emit r0
        let inst1 = &func.instructions[1];
        assert_eq!(inst1.opcode(), Some(OpCode::Emit));
        assert_eq!(inst1.a(), 0); // r0

        // Third instruction: Halt
        let inst2 = &func.instructions[2];
        assert_eq!(inst2.opcode(), Some(OpCode::Halt));
    }

    #[test]
    fn test_compile_arithmetic() {
        let module = compile("let x = 1 + 2\nemit x").unwrap();
        let func = &module.functions[0];

        // LoadConst r0, K0 (1)
        // LoadConst r1, K1 (2)
        // Add r2, r0, r1
        // Emit r2
        // Halt
        assert_eq!(func.instructions[2].opcode(), Some(OpCode::Add));
    }

    #[test]
    fn test_compile_comparison() {
        let module = compile("let x = 5 > 3\nemit x").unwrap();
        let func = &module.functions[0];
        // LoadConst, LoadConst, Gt, Emit, Halt
        assert_eq!(func.instructions[2].opcode(), Some(OpCode::Gt));
    }

    #[test]
    fn test_compile_bool_literals() {
        let module = compile("let x = true\nlet y = false").unwrap();
        let func = &module.functions[0];
        assert_eq!(func.instructions[0].opcode(), Some(OpCode::LoadTrue));
        assert_eq!(func.instructions[1].opcode(), Some(OpCode::LoadFalse));
    }
}
