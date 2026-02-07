use std::collections::{HashMap, VecDeque};
use agentus_ir::module::{Constant, Module};
use agentus_ir::opcode::OpCode;
use crate::host::{ExecRequest, HostInterface, NoHost, ToolCallRequest};
use crate::value::Value;

/// Output handler for the VM.
pub trait OutputHandler {
    fn on_emit(&self, value: &Value);
    fn on_log(&self, level: u8, message: &str);
}

/// Default output handler that prints to stdout.
pub struct StdoutHandler;

impl OutputHandler for StdoutHandler {
    fn on_emit(&self, value: &Value) {
        println!("{}", value);
    }

    fn on_log(&self, level: u8, message: &str) {
        let level_str = match level {
            0 => "TRACE",
            1 => "DEBUG",
            2 => "INFO",
            3 => "WARN",
            4 => "ERROR",
            _ => "LOG",
        };
        eprintln!("[{}] {}", level_str, message);
    }
}

/// A live agent instance with persistent memory.
struct AgentInstance {
    /// Index into the module's agent descriptor table.
    descriptor_idx: u32,
    /// Persistent memory fields: field_name -> value.
    memory: HashMap<String, Value>,
    /// Message mailbox for inter-agent communication.
    mailbox: VecDeque<Value>,
}

/// A call frame / activation record.
struct CallFrame {
    /// Register file (up to 256 slots).
    registers: Vec<Value>,
    /// Function index in the module.
    function_idx: u32,
    /// Program counter (instruction index).
    pc: usize,
    /// Return address: (function_idx, pc, return_register) of the caller.
    return_info: Option<(u32, usize, u8)>,
    /// Which agent instance this frame belongs to (for method calls).
    agent_id: Option<u64>,
}

/// The Agentus Virtual Machine.
pub struct VM {
    module: Module,
    /// The call stack.
    call_stack: Vec<CallFrame>,
    /// Output handler.
    output: Box<dyn OutputHandler>,
    /// Collected emit outputs (for testing).
    outputs: Vec<Value>,
    /// Live agent instances.
    agents: HashMap<u64, AgentInstance>,
    /// Next agent instance ID.
    next_agent_id: u64,
    /// Host interface for LLM execution.
    host: Box<dyn HostInterface>,
}

impl VM {
    pub fn new(module: Module) -> Self {
        Self {
            module,
            call_stack: Vec::new(),
            output: Box::new(StdoutHandler),
            outputs: Vec::new(),
            agents: HashMap::new(),
            next_agent_id: 1,
            host: Box::new(NoHost),
        }
    }

    pub fn with_output(mut self, handler: Box<dyn OutputHandler>) -> Self {
        self.output = handler;
        self
    }

    pub fn with_host(mut self, host: Box<dyn HostInterface>) -> Self {
        self.host = host;
        self
    }

    /// Get all emitted outputs (for testing).
    pub fn get_outputs(&self) -> &[Value] {
        &self.outputs
    }

    /// Run the module from its entry function.
    pub fn run(&mut self) -> Result<(), String> {
        let entry = self.module.entry_function;
        self.push_frame(entry, Option::None)?;
        self.execute()
    }

    fn push_frame(
        &mut self,
        function_idx: u32,
        return_info: Option<(u32, usize, u8)>,
    ) -> Result<(), String> {
        self.push_frame_with_agent(function_idx, return_info, None)
    }

    fn push_frame_with_agent(
        &mut self,
        function_idx: u32,
        return_info: Option<(u32, usize, u8)>,
        agent_id: Option<u64>,
    ) -> Result<(), String> {
        let func = self
            .module
            .get_function(function_idx)
            .ok_or_else(|| format!("function {} not found", function_idx))?;

        let registers = vec![Value::None; func.num_registers as usize];

        self.call_stack.push(CallFrame {
            registers,
            function_idx,
            pc: 0,
            return_info,
            agent_id,
        });

        Ok(())
    }

    fn execute(&mut self) -> Result<(), String> {
        loop {
            if self.call_stack.is_empty() {
                return Ok(());
            }

            let frame = self.call_stack.last().unwrap();
            let func_idx = frame.function_idx;
            let pc = frame.pc;

            let func = self
                .module
                .get_function(func_idx)
                .ok_or("invalid function index")?;

            if pc >= func.instructions.len() {
                // Function ended without explicit return
                self.call_stack.pop();
                continue;
            }

            let inst = func.instructions[pc];
            let opcode = inst
                .opcode()
                .ok_or_else(|| format!("invalid opcode 0x{:02X} at pc={}", inst.opcode_byte(), pc))?;

            // Advance PC before executing (some instructions modify it)
            self.call_stack.last_mut().unwrap().pc += 1;

            match opcode {
                OpCode::Nop => {}
                OpCode::Halt => {
                    return Ok(());
                }

                // Load / Store / Move
                OpCode::LoadConst => {
                    let a = inst.a() as usize;
                    let bx = inst.bx();
                    let value = self.load_constant(bx)?;
                    self.set_register(a, value);
                }
                OpCode::LoadNone => {
                    let a = inst.a() as usize;
                    self.set_register(a, Value::None);
                }
                OpCode::LoadTrue => {
                    let a = inst.a() as usize;
                    self.set_register(a, Value::Bool(true));
                }
                OpCode::LoadFalse => {
                    let a = inst.a() as usize;
                    self.set_register(a, Value::Bool(false));
                }
                OpCode::Move => {
                    let a = inst.a() as usize;
                    let b = inst.b() as usize;
                    let value = self.get_register(b).clone();
                    self.set_register(a, value);
                }

                // Arithmetic
                OpCode::Add => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.arith_op(b, c, |x, y| x + y)?;
                    self.set_register(a, result);
                }
                OpCode::Sub => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.arith_op(b, c, |x, y| x - y)?;
                    self.set_register(a, result);
                }
                OpCode::Mul => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.arith_op(b, c, |x, y| x * y)?;
                    self.set_register(a, result);
                }
                OpCode::Div => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.arith_op(b, c, |x, y| x / y)?;
                    self.set_register(a, result);
                }
                OpCode::Mod => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.arith_op(b, c, |x, y| x % y)?;
                    self.set_register(a, result);
                }
                OpCode::Neg => {
                    let (a, b) = (inst.a() as usize, inst.b() as usize);
                    let val = self.get_register(b);
                    match val {
                        Value::Num(n) => self.set_register(a, Value::Num(-n)),
                        _ => return Err("Neg requires numeric operand".to_string()),
                    }
                }

                // Comparison
                OpCode::Eq => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.get_register(b) == self.get_register(c);
                    self.set_register(a, Value::Bool(result));
                }
                OpCode::Neq => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.get_register(b) != self.get_register(c);
                    self.set_register(a, Value::Bool(result));
                }
                OpCode::Lt => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.cmp_op(b, c, |x, y| x < y)?;
                    self.set_register(a, result);
                }
                OpCode::Lte => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.cmp_op(b, c, |x, y| x <= y)?;
                    self.set_register(a, result);
                }
                OpCode::Gt => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.cmp_op(b, c, |x, y| x > y)?;
                    self.set_register(a, result);
                }
                OpCode::Gte => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let result = self.cmp_op(b, c, |x, y| x >= y)?;
                    self.set_register(a, result);
                }

                // Logic
                OpCode::And => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let lhs = self.get_register(b).is_truthy();
                    let rhs = self.get_register(c).is_truthy();
                    self.set_register(a, Value::Bool(lhs && rhs));
                }
                OpCode::Or => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let lhs = self.get_register(b).is_truthy();
                    let rhs = self.get_register(c).is_truthy();
                    self.set_register(a, Value::Bool(lhs || rhs));
                }
                OpCode::Not => {
                    let (a, b) = (inst.a() as usize, inst.b() as usize);
                    let val = self.get_register(b).is_truthy();
                    self.set_register(a, Value::Bool(!val));
                }

                // String
                OpCode::Concat => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let lhs = self.get_register(b).to_string();
                    let rhs = self.get_register(c).to_string();
                    self.set_register(a, Value::from_string(format!("{}{}", lhs, rhs)));
                }

                // Control flow
                OpCode::Jmp => {
                    let offset = inst.sbx_24();
                    let frame = self.call_stack.last_mut().unwrap();
                    frame.pc = (frame.pc as i32 + offset) as usize;
                }
                OpCode::JmpTrue => {
                    let a = inst.a() as usize;
                    let offset = inst.sbx_16();
                    if self.get_register(a).is_truthy() {
                        let frame = self.call_stack.last_mut().unwrap();
                        frame.pc = (frame.pc as i32 + offset as i32) as usize;
                    }
                }
                OpCode::JmpFalse => {
                    let a = inst.a() as usize;
                    let offset = inst.sbx_16();
                    if !self.get_register(a).is_truthy() {
                        let frame = self.call_stack.last_mut().unwrap();
                        frame.pc = (frame.pc as i32 + offset as i32) as usize;
                    }
                }

                // I/O
                OpCode::Emit => {
                    let a = inst.a() as usize;
                    let value = self.get_register(a).clone();
                    self.output.on_emit(&value);
                    self.outputs.push(value);
                }
                OpCode::Log => {
                    let level = inst.b();
                    let c = inst.c() as usize;
                    let msg = self.get_register(c).to_string();
                    self.output.on_log(level, &msg);
                }

                // Function call
                OpCode::Call => {
                    let result_reg = inst.a();
                    let func_idx_raw = inst.bx();

                    if func_idx_raw == 0xFFFE {
                        // Method call dispatch (sentinel)
                        let frame = self.call_stack.last().unwrap();
                        let pc1 = frame.pc;
                        let func = self.module.get_function(frame.function_idx)
                            .ok_or("invalid function index")?;
                        let extra1 = func.instructions[pc1];
                        let extra2 = func.instructions[pc1 + 1];
                        self.call_stack.last_mut().unwrap().pc += 2;

                        let first_arg_reg = extra1.b() as usize;
                        let num_args = extra1.c() as usize;
                        let method_name_idx = extra2.bx();

                        let method_name = self.load_constant_str(method_name_idx)?;

                        // r(first_arg_reg) is the receiver
                        let handle = self.get_register(first_arg_reg).clone();

                        // Built-in collection methods
                        match &handle {
                            Value::List(list) => {
                                match method_name.as_str() {
                                    "push" => {
                                        if num_args < 2 {
                                            return Err("list.push() requires an argument".to_string());
                                        }
                                        let val = self.get_register(first_arg_reg + 1).clone();
                                        list.borrow_mut().push(val);
                                        self.set_register(result_reg as usize, Value::None);
                                        continue;
                                    }
                                    "len" => {
                                        let len = list.borrow().len();
                                        self.set_register(result_reg as usize, Value::Num(len as f64));
                                        continue;
                                    }
                                    _ => return Err(format!("unknown list method '{}'", method_name)),
                                }
                            }
                            Value::Map(map) => {
                                match method_name.as_str() {
                                    "len" => {
                                        let len = map.borrow().len();
                                        self.set_register(result_reg as usize, Value::Num(len as f64));
                                        continue;
                                    }
                                    "keys" => {
                                        let keys: Vec<Value> = map.borrow().keys()
                                            .map(|k| Value::from_str(k))
                                            .collect();
                                        self.set_register(result_reg as usize, Value::List(std::rc::Rc::new(std::cell::RefCell::new(keys))));
                                        continue;
                                    }
                                    "values" => {
                                        let vals: Vec<Value> = map.borrow().values()
                                            .cloned()
                                            .collect();
                                        self.set_register(result_reg as usize, Value::List(std::rc::Rc::new(std::cell::RefCell::new(vals))));
                                        continue;
                                    }
                                    "contains" => {
                                        if num_args < 2 {
                                            return Err("map.contains() requires an argument".to_string());
                                        }
                                        let key = self.get_register(first_arg_reg + 1).to_string();
                                        let has = map.borrow().contains_key(&key);
                                        self.set_register(result_reg as usize, Value::Bool(has));
                                        continue;
                                    }
                                    "remove" => {
                                        if num_args < 2 {
                                            return Err("map.remove() requires an argument".to_string());
                                        }
                                        let key = self.get_register(first_arg_reg + 1).to_string();
                                        let removed = map.borrow_mut().remove(&key).unwrap_or(Value::None);
                                        self.set_register(result_reg as usize, removed);
                                        continue;
                                    }
                                    _ => return Err(format!("unknown map method '{}'", method_name)),
                                }
                            }
                            Value::Str(s) => {
                                match method_name.as_str() {
                                    "len" => {
                                        self.set_register(result_reg as usize, Value::Num(s.len() as f64));
                                        continue;
                                    }
                                    _ => return Err(format!("unknown string method '{}'", method_name)),
                                }
                            }
                            _ => {}
                        }

                        let agent_id = match &handle {
                            Value::AgentHandle(id) => *id,
                            _ => return Err(format!("method call on non-agent: {}", handle)),
                        };

                        let agent = self.agents.get(&agent_id)
                            .ok_or_else(|| format!("agent {} not found", agent_id))?;
                        let desc_idx = agent.descriptor_idx;
                        let descriptor = self.module.get_agent(desc_idx)
                            .ok_or_else(|| format!("agent descriptor {} not found", desc_idx))?
                            .clone();

                        // Find method by name
                        let method_func_idx = descriptor.methods.iter()
                            .find(|(name_idx, _)| {
                                self.load_constant_str(*name_idx).ok().as_deref() == Some(method_name.as_str())
                            })
                            .map(|(_, idx)| *idx)
                            .ok_or_else(|| format!("method '{}' not found on agent", method_name))?;

                        // Collect arguments (skip the handle at first_arg_reg)
                        let mut arg_values = Vec::with_capacity(if num_args > 0 { num_args - 1 } else { 0 });
                        for i in 1..num_args {
                            arg_values.push(self.get_register(first_arg_reg + i).clone());
                        }

                        let caller_func_idx = self.call_stack.last().unwrap().function_idx;
                        let caller_pc = self.call_stack.last().unwrap().pc;
                        let return_info = Some((caller_func_idx, caller_pc, result_reg));

                        self.push_frame_with_agent(method_func_idx, return_info, Some(agent_id))?;

                        // Copy arguments (params are r0, r1, ...)
                        for (i, val) in arg_values.into_iter().enumerate() {
                            self.set_register(i, val);
                        }
                    } else {
                        // Regular function call
                        let func_idx = func_idx_raw as u32;

                        // Read the extra data word (next instruction)
                        let frame = self.call_stack.last().unwrap();
                        let extra_pc = frame.pc;
                        let func = self.module.get_function(frame.function_idx)
                            .ok_or("invalid function index")?;
                        let extra = func.instructions[extra_pc];
                        // Advance PC past the extra word
                        self.call_stack.last_mut().unwrap().pc += 1;

                        let first_arg_reg = extra.b() as usize;
                        let num_args = extra.c() as usize;

                        // Collect argument values from caller's registers
                        let mut arg_values = Vec::with_capacity(num_args);
                        for i in 0..num_args {
                            arg_values.push(self.get_register(first_arg_reg + i).clone());
                        }

                        // Save return info
                        let caller_func_idx = self.call_stack.last().unwrap().function_idx;
                        let caller_pc = self.call_stack.last().unwrap().pc;
                        let return_info = Some((caller_func_idx, caller_pc, result_reg));

                        // Push new frame
                        self.push_frame(func_idx, return_info)?;

                        // Copy arguments into the new frame's registers
                        for (i, val) in arg_values.into_iter().enumerate() {
                            self.set_register(i, val);
                        }
                    }
                }

                // Return
                OpCode::Ret => {
                    let a = inst.a() as usize;
                    let return_value = self.get_register(a).clone();
                    let frame = self.call_stack.pop().unwrap();
                    if let Some((_func_idx, _pc, ret_reg)) = frame.return_info {
                        self.set_register(ret_reg as usize, return_value);
                    }
                }
                OpCode::RetNone => {
                    let frame = self.call_stack.pop().unwrap();
                    if let Some((_func_idx, _pc, ret_reg)) = frame.return_info {
                        self.set_register(ret_reg as usize, Value::None);
                    }
                }

                // Collections
                OpCode::NewList => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let mut items = Vec::with_capacity(c);
                    for i in 0..c {
                        items.push(self.get_register(b + i).clone());
                    }
                    self.set_register(a, Value::List(std::rc::Rc::new(std::cell::RefCell::new(items))));
                }
                OpCode::NewMap => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let mut map = std::collections::HashMap::new();
                    for i in 0..c {
                        let key = self.get_register(b + i * 2).to_string();
                        let val = self.get_register(b + i * 2 + 1).clone();
                        map.insert(key, val);
                    }
                    self.set_register(a, Value::Map(std::rc::Rc::new(std::cell::RefCell::new(map))));
                }
                OpCode::IndexGet => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let obj = self.get_register(b).clone();
                    let idx = self.get_register(c).clone();
                    let result = match (&obj, &idx) {
                        (Value::List(list), Value::Num(n)) => {
                            let i = *n as usize;
                            let items = list.borrow();
                            items.get(i).cloned().unwrap_or(Value::None)
                        }
                        (Value::Map(map), Value::Str(key)) => {
                            let items = map.borrow();
                            items.get(key.as_str()).cloned().unwrap_or(Value::None)
                        }
                        _ => return Err(format!("cannot index {:?} with {:?}", obj, idx)),
                    };
                    self.set_register(a, result);
                }

                OpCode::IndexSet => {
                    let (a, b, c) = (inst.a() as usize, inst.b() as usize, inst.c() as usize);
                    let idx_val = self.get_register(b).clone();
                    let val = self.get_register(c).clone();
                    let obj = self.get_register(a).clone();
                    match (&obj, &idx_val) {
                        (Value::List(list), Value::Num(n)) => {
                            let i = *n as usize;
                            let mut items = list.borrow_mut();
                            if i < items.len() {
                                items[i] = val;
                            } else {
                                return Err(format!("list index {} out of bounds", i));
                            }
                        }
                        (Value::Map(map), Value::Str(key)) => {
                            map.borrow_mut().insert(key.to_string(), val);
                        }
                        _ => return Err(format!("cannot index-set {:?} with {:?}", obj, idx_val)),
                    }
                }
                OpCode::Len => {
                    let (a, b) = (inst.a() as usize, inst.b() as usize);
                    let obj = self.get_register(b).clone();
                    let len = match &obj {
                        Value::List(l) => l.borrow().len(),
                        Value::Map(m) => m.borrow().len(),
                        Value::Str(s) => s.len(),
                        _ => return Err(format!("cannot get length of {:?}", obj)),
                    };
                    self.set_register(a, Value::Num(len as f64));
                }
                OpCode::ListPush => {
                    let (a, b) = (inst.a() as usize, inst.b() as usize);
                    let val = self.get_register(b).clone();
                    let list = self.get_register(a).clone();
                    match &list {
                        Value::List(l) => l.borrow_mut().push(val),
                        _ => return Err(format!("cannot push to {:?}", list)),
                    }
                }
                OpCode::StrLen => {
                    let (a, b) = (inst.a() as usize, inst.b() as usize);
                    let val = self.get_register(b).clone();
                    match &val {
                        Value::Str(s) => self.set_register(a, Value::Num(s.len() as f64)),
                        _ => return Err(format!("StrLen requires string, got {:?}", val)),
                    }
                }

                // Iterators
                OpCode::IterInit => {
                    let (a, b) = (inst.a() as usize, inst.b() as usize);
                    let source = self.get_register(b).clone();
                    let items = match &source {
                        Value::List(l) => l.borrow().clone(),
                        Value::Map(m) => {
                            // Iterate over keys
                            m.borrow().keys().map(|k| Value::from_str(k)).collect()
                        }
                        _ => return Err(format!("cannot iterate over {:?}", source)),
                    };
                    self.set_register(
                        a,
                        Value::Iterator(std::rc::Rc::new(std::cell::RefCell::new((items, 0)))),
                    );
                }
                OpCode::IterNext => {
                    // Two-instruction sequence:
                    // 1. IterNext A=var_reg, sBx=jump_offset_if_exhausted
                    // 2. Extra data: B=iter_reg
                    let var_reg = inst.a() as usize;
                    let jump_offset = inst.sbx_16();

                    // Read extra data word
                    let frame = self.call_stack.last().unwrap();
                    let extra_pc = frame.pc;
                    let func = self.module.get_function(frame.function_idx)
                        .ok_or("invalid function index")?;
                    let extra = func.instructions[extra_pc];
                    self.call_stack.last_mut().unwrap().pc += 1;

                    let iter_reg = extra.b() as usize;

                    let iter_val = self.get_register(iter_reg).clone();
                    match &iter_val {
                        Value::Iterator(state) => {
                            let mut st = state.borrow_mut();
                            if st.1 < st.0.len() {
                                let val = st.0[st.1].clone();
                                st.1 += 1;
                                drop(st);
                                self.set_register(var_reg, val);
                            } else {
                                drop(st);
                                // Iterator exhausted — jump
                                let frame = self.call_stack.last_mut().unwrap();
                                frame.pc = (frame.pc as i32 + jump_offset as i32) as usize;
                            }
                        }
                        _ => return Err(format!("IterNext on non-iterator: {:?}", iter_val)),
                    }
                }

                // Agent memory
                OpCode::MLoad => {
                    let a = inst.a() as usize;
                    let bx = inst.bx();
                    let field_name = self.load_constant_str(bx)?;
                    let agent_id = self.current_agent_id()?;
                    let agent = self.agents.get(&agent_id)
                        .ok_or_else(|| format!("agent {} not found", agent_id))?;
                    let value = agent.memory.get(&field_name)
                        .cloned()
                        .unwrap_or(Value::None);
                    self.set_register(a, value);
                }
                OpCode::MStore => {
                    let a = inst.a() as usize;
                    let bx = inst.bx();
                    let field_name = self.load_constant_str(bx)?;
                    let value = self.get_register(a).clone();
                    let agent_id = self.current_agent_id()?;
                    let agent = self.agents.get_mut(&agent_id)
                        .ok_or_else(|| format!("agent {} not found", agent_id))?;
                    agent.memory.insert(field_name, value);
                }

                // Agent spawn
                OpCode::Spawn => {
                    let a = inst.a() as usize;
                    let bx = inst.bx() as u32;
                    let descriptor = self.module.get_agent(bx)
                        .ok_or_else(|| format!("agent descriptor {} not found", bx))?
                        .clone();

                    // Initialize memory with defaults
                    let mut memory = HashMap::new();
                    for field in &descriptor.memory_fields {
                        let name = self.load_constant_str(field.name_idx)?;
                        let default_val = if let Some(default_idx) = field.default_idx {
                            self.load_constant(default_idx)?
                        } else {
                            Value::None
                        };
                        memory.insert(name, default_val);
                    }

                    let id = self.next_agent_id;
                    self.next_agent_id += 1;
                    self.agents.insert(id, AgentInstance {
                        descriptor_idx: bx,
                        memory,
                        mailbox: VecDeque::new(),
                    });
                    self.set_register(a, Value::AgentHandle(id));
                }

                // LLM execution
                OpCode::Exec => {
                    let a = inst.a() as usize;
                    let b = inst.b() as usize;
                    let prompt = self.get_register(b).to_string();

                    // Get model/system_prompt from agent context if available
                    let (model, sys_prompt) = self.get_agent_context();

                    let request = ExecRequest {
                        model: model.unwrap_or_else(|| "default".to_string()),
                        system_prompt: sys_prompt,
                        user_prompt: prompt,
                    };
                    let result = self.host.exec(request).map_err(|e| format!("exec error: {}", e))?;
                    self.set_register(a, Value::from_string(result));
                }

                // Agent message passing
                OpCode::Send => {
                    let a = inst.a() as usize;
                    let b = inst.b() as usize;
                    let handle = self.get_register(a).clone();
                    let message = self.get_register(b).clone();
                    let agent_id = match &handle {
                        Value::AgentHandle(id) => *id,
                        _ => return Err(format!("send target is not an agent handle: {}", handle)),
                    };
                    let agent = self.agents.get_mut(&agent_id)
                        .ok_or_else(|| format!("agent {} not found", agent_id))?;
                    agent.mailbox.push_back(message);
                }
                OpCode::Recv => {
                    let a = inst.a() as usize;
                    let b = inst.b() as usize;
                    let handle = self.get_register(b).clone();
                    let agent_id = match &handle {
                        Value::AgentHandle(id) => *id,
                        _ => return Err(format!("recv target is not an agent handle: {}", handle)),
                    };
                    let agent = self.agents.get_mut(&agent_id)
                        .ok_or_else(|| format!("agent {} not found", agent_id))?;
                    let value = agent.mailbox.pop_front().unwrap_or(Value::None);
                    self.set_register(a, value);
                }

                // Tool call
                OpCode::TCall => {
                    let result_reg = inst.a() as usize;
                    let tool_desc_idx = inst.bx() as u32;

                    // Read the extra data word (next instruction)
                    let frame = self.call_stack.last().unwrap();
                    let extra_pc = frame.pc;
                    let func = self.module.get_function(frame.function_idx)
                        .ok_or("invalid function index")?;
                    let extra = func.instructions[extra_pc];
                    self.call_stack.last_mut().unwrap().pc += 1;

                    let first_arg_reg = extra.b() as usize;
                    let num_args = extra.c() as usize;

                    // Get tool descriptor
                    let tool_desc = self.module.get_tool(tool_desc_idx)
                        .ok_or_else(|| format!("tool descriptor {} not found", tool_desc_idx))?
                        .clone();

                    let tool_name = self.load_constant_str(tool_desc.name_idx)?;

                    // Build named arguments from registers + param names
                    let mut args = Vec::new();
                    for i in 0..num_args {
                        let param_name = if i < tool_desc.params.len() {
                            self.load_constant_str(tool_desc.params[i].name_idx)?
                        } else {
                            format!("arg{}", i)
                        };
                        let value = self.get_register(first_arg_reg + i).to_string();
                        args.push((param_name, value));
                    }

                    let request = ToolCallRequest {
                        tool_name,
                        args,
                    };
                    let result = self.host.tool_call(request)
                        .map_err(|e| format!("tool call error: {}", e))?;
                    self.set_register(result_reg, Value::from_string(result));
                }

                // Stubs for not-yet-implemented opcodes
                _ => {
                    return Err(format!("opcode {:?} not yet implemented", opcode));
                }
            }
        }
    }

    // =====================================================================
    // Helpers
    // =====================================================================

    fn get_register(&self, idx: usize) -> &Value {
        let frame = self.call_stack.last().unwrap();
        &frame.registers[idx]
    }

    fn set_register(&mut self, idx: usize, value: Value) {
        let frame = self.call_stack.last_mut().unwrap();
        if idx >= frame.registers.len() {
            frame.registers.resize(idx + 1, Value::None);
        }
        frame.registers[idx] = value;
    }

    fn load_constant(&self, idx: u16) -> Result<Value, String> {
        let constant = self
            .module
            .get_constant(idx)
            .ok_or_else(|| format!("constant {} not found", idx))?;
        Ok(match constant {
            Constant::None => Value::None,
            Constant::Bool(b) => Value::Bool(*b),
            Constant::Num(n) => Value::Num(*n),
            Constant::Str(s) => Value::from_str(s),
        })
    }

    fn arith_op(
        &self,
        b: usize,
        c: usize,
        op: fn(f64, f64) -> f64,
    ) -> Result<Value, String> {
        let lhs = self.get_register(b);
        let rhs = self.get_register(c);
        match (lhs, rhs) {
            (Value::Num(a), Value::Num(b)) => Ok(Value::Num(op(*a, *b))),
            _ => Err(format!(
                "arithmetic requires numeric operands, got {} and {}",
                lhs, rhs
            )),
        }
    }

    fn cmp_op(
        &self,
        b: usize,
        c: usize,
        op: fn(f64, f64) -> bool,
    ) -> Result<Value, String> {
        let lhs = self.get_register(b);
        let rhs = self.get_register(c);
        match (lhs, rhs) {
            (Value::Num(a), Value::Num(b)) => Ok(Value::Bool(op(*a, *b))),
            _ => Err(format!(
                "comparison requires numeric operands, got {} and {}",
                lhs, rhs
            )),
        }
    }

    fn load_constant_str(&self, idx: u16) -> Result<String, String> {
        let constant = self
            .module
            .get_constant(idx)
            .ok_or_else(|| format!("constant {} not found", idx))?;
        match constant {
            Constant::Str(s) => Ok(s.clone()),
            _ => Err(format!("expected string constant at index {}", idx)),
        }
    }

    fn current_agent_id(&self) -> Result<u64, String> {
        self.call_stack
            .last()
            .and_then(|f| f.agent_id)
            .ok_or_else(|| "not in an agent context".to_string())
    }

    fn get_agent_context(&self) -> (Option<String>, Option<String>) {
        let agent_id = self.call_stack.last().and_then(|f| f.agent_id);
        if let Some(id) = agent_id {
            if let Some(agent) = self.agents.get(&id) {
                let desc = self.module.get_agent(agent.descriptor_idx);
                if let Some(desc) = desc {
                    let model = desc.model_idx.and_then(|idx| {
                        self.load_constant_str(idx).ok()
                    });
                    let sys = desc.system_prompt_idx.and_then(|idx| {
                        self.load_constant_str(idx).ok()
                    });
                    return (model, sys);
                }
            }
        }
        (None, None)
    }
}

/// Convenience: no-op output handler for testing.
pub struct SilentHandler;

impl OutputHandler for SilentHandler {
    fn on_emit(&self, _: &Value) {}
    fn on_log(&self, _: u8, _: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentus_ir::instruction::Instruction;
    use agentus_ir::module::{Constant, Function, Module};
    use agentus_ir::opcode::OpCode;

    fn make_module(constants: Vec<Constant>, instructions: Vec<Instruction>) -> Module {
        Module {
            constants,
            functions: vec![Function {
                name_idx: 0,
                num_params: 0,
                num_registers: 16,
                instructions,
            }],
            agents: Vec::new(),
            tools: Vec::new(),
            entry_function: 0,
        }
    }

    #[test]
    fn test_load_const_and_emit() {
        let module = make_module(
            vec![Constant::Str("Hello Agentus".to_string())],
            vec![
                Instruction::abx(OpCode::LoadConst, 0, 0),
                Instruction::op_a(OpCode::Emit, 0),
                Instruction::op_only(OpCode::Halt),
            ],
        );

        let mut vm = VM::new(module).with_output(Box::new(SilentHandler));
        vm.run().unwrap();
        assert_eq!(vm.outputs.len(), 1);
        assert_eq!(vm.outputs[0], Value::from_str("Hello Agentus"));
    }

    #[test]
    fn test_arithmetic() {
        let module = make_module(
            vec![Constant::Num(10.0), Constant::Num(3.0)],
            vec![
                Instruction::abx(OpCode::LoadConst, 0, 0), // r0 = 10
                Instruction::abx(OpCode::LoadConst, 1, 1), // r1 = 3
                Instruction::abc(OpCode::Add, 2, 0, 1),    // r2 = r0 + r1
                Instruction::op_a(OpCode::Emit, 2),
                Instruction::op_only(OpCode::Halt),
            ],
        );

        let mut vm = VM::new(module).with_output(Box::new(SilentHandler));
        vm.run().unwrap();
        assert_eq!(vm.outputs[0], Value::Num(13.0));
    }

    #[test]
    fn test_comparison() {
        let module = make_module(
            vec![Constant::Num(5.0), Constant::Num(3.0)],
            vec![
                Instruction::abx(OpCode::LoadConst, 0, 0),
                Instruction::abx(OpCode::LoadConst, 1, 1),
                Instruction::abc(OpCode::Gt, 2, 0, 1), // r2 = 5 > 3
                Instruction::op_a(OpCode::Emit, 2),
                Instruction::op_only(OpCode::Halt),
            ],
        );

        let mut vm = VM::new(module).with_output(Box::new(SilentHandler));
        vm.run().unwrap();
        assert_eq!(vm.outputs[0], Value::Bool(true));
    }

    #[test]
    fn test_conditional_jump() {
        // if true { emit "yes" } else { emit "no" }
        let module = make_module(
            vec![
                Constant::Str("yes".to_string()),
                Constant::Str("no".to_string()),
            ],
            vec![
                Instruction::op_a(OpCode::LoadTrue, 0),           // r0 = true
                Instruction::asbx(OpCode::JmpFalse, 0, 2),       // if !r0, skip 2
                Instruction::abx(OpCode::LoadConst, 1, 0),        // r1 = "yes"
                Instruction::sbx(OpCode::Jmp, 1),                 // skip else
                Instruction::abx(OpCode::LoadConst, 1, 1),        // r1 = "no"
                Instruction::op_a(OpCode::Emit, 1),               // emit r1
                Instruction::op_only(OpCode::Halt),
            ],
        );

        let mut vm = VM::new(module).with_output(Box::new(SilentHandler));
        vm.run().unwrap();
        assert_eq!(vm.outputs[0], Value::from_str("yes"));
    }

    #[test]
    fn test_string_concat() {
        let module = make_module(
            vec![
                Constant::Str("Hello ".to_string()),
                Constant::Str("World".to_string()),
            ],
            vec![
                Instruction::abx(OpCode::LoadConst, 0, 0),
                Instruction::abx(OpCode::LoadConst, 1, 1),
                Instruction::abc(OpCode::Concat, 2, 0, 1),
                Instruction::op_a(OpCode::Emit, 2),
                Instruction::op_only(OpCode::Halt),
            ],
        );

        let mut vm = VM::new(module).with_output(Box::new(SilentHandler));
        vm.run().unwrap();
        assert_eq!(vm.outputs[0], Value::from_str("Hello World"));
    }

    #[test]
    fn test_bool_logic() {
        let module = make_module(
            vec![],
            vec![
                Instruction::op_a(OpCode::LoadTrue, 0),
                Instruction::op_a(OpCode::LoadFalse, 1),
                Instruction::abc(OpCode::And, 2, 0, 1), // true && false = false
                Instruction::op_a(OpCode::Emit, 2),
                Instruction::abc(OpCode::Or, 3, 0, 1), // true || false = true
                Instruction::op_a(OpCode::Emit, 3),
                Instruction::op_only(OpCode::Halt),
            ],
        );

        let mut vm = VM::new(module).with_output(Box::new(SilentHandler));
        vm.run().unwrap();
        assert_eq!(vm.outputs[0], Value::Bool(false));
        assert_eq!(vm.outputs[1], Value::Bool(true));
    }
}
