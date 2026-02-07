use crate::instruction::Instruction;

/// A constant value in the constant pool.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    None,
    Bool(bool),
    Num(f64),
    Str(String),
}

/// A compiled function.
#[derive(Debug, Clone)]
pub struct Function {
    /// Index into the constant pool for the function name.
    pub name_idx: u32,
    /// Number of parameters.
    pub num_params: u8,
    /// Number of registers needed (max register index + 1).
    pub num_registers: u8,
    /// The bytecode instructions for this function.
    pub instructions: Vec<Instruction>,
}

/// Describes an agent type in the module.
#[derive(Debug, Clone)]
pub struct AgentDescriptor {
    /// Index into the constant pool for the agent name.
    pub name_idx: u16,
    /// Index into the constant pool for the model string (optional).
    pub model_idx: Option<u16>,
    /// Index into the constant pool for the system prompt string (optional).
    pub system_prompt_idx: Option<u16>,
    /// Agent memory field descriptors.
    pub memory_fields: Vec<AgentMemoryField>,
    /// Methods: (name_const_idx, function_table_idx).
    pub methods: Vec<(u16, u32)>,
}

/// A single memory field in an agent descriptor.
#[derive(Debug, Clone)]
pub struct AgentMemoryField {
    /// Index into the constant pool for the field name.
    pub name_idx: u16,
    /// Index into the constant pool for the default value (optional, simple literals only).
    pub default_idx: Option<u16>,
}

/// A compiled module — the output of the compiler, input to the runtime.
#[derive(Debug, Clone)]
pub struct Module {
    /// Constant pool: strings, numbers, booleans referenced by instructions.
    pub constants: Vec<Constant>,
    /// Function table.
    pub functions: Vec<Function>,
    /// Agent descriptor table.
    pub agents: Vec<AgentDescriptor>,
    /// Index of the entry point function (usually `main` or the top-level script).
    pub entry_function: u32,
}

impl Module {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            functions: Vec::new(),
            agents: Vec::new(),
            entry_function: 0,
        }
    }

    /// Add a constant and return its index.
    pub fn add_constant(&mut self, constant: Constant) -> u16 {
        // Check if constant already exists (dedup)
        for (i, existing) in self.constants.iter().enumerate() {
            if *existing == constant {
                return i as u16;
            }
        }
        let idx = self.constants.len();
        assert!(idx <= u16::MAX as usize, "constant pool overflow");
        self.constants.push(constant);
        idx as u16
    }

    /// Add a function and return its index.
    pub fn add_function(&mut self, function: Function) -> u32 {
        let idx = self.functions.len();
        self.functions.push(function);
        idx as u32
    }

    /// Get a constant by index.
    pub fn get_constant(&self, idx: u16) -> Option<&Constant> {
        self.constants.get(idx as usize)
    }

    /// Get a function by index.
    pub fn get_function(&self, idx: u32) -> Option<&Function> {
        self.functions.get(idx as usize)
    }

    /// Add an agent descriptor and return its index.
    pub fn add_agent(&mut self, agent: AgentDescriptor) -> u32 {
        let idx = self.agents.len();
        self.agents.push(agent);
        idx as u32
    }

    /// Get an agent descriptor by index.
    pub fn get_agent(&self, idx: u32) -> Option<&AgentDescriptor> {
        self.agents.get(idx as usize)
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a Module incrementally during compilation.
#[derive(Debug)]
pub struct ModuleBuilder {
    module: Module,
}

impl ModuleBuilder {
    pub fn new() -> Self {
        Self {
            module: Module::new(),
        }
    }

    pub fn add_string_constant(&mut self, s: &str) -> u16 {
        self.module.add_constant(Constant::Str(s.to_string()))
    }

    pub fn add_num_constant(&mut self, n: f64) -> u16 {
        self.module.add_constant(Constant::Num(n))
    }

    pub fn add_bool_constant(&mut self, b: bool) -> u16 {
        self.module.add_constant(Constant::Bool(b))
    }

    pub fn add_none_constant(&mut self) -> u16 {
        self.module.add_constant(Constant::None)
    }

    pub fn add_function(&mut self, function: Function) -> u32 {
        self.module.add_function(function)
    }

    pub fn add_agent(&mut self, agent: AgentDescriptor) -> u32 {
        self.module.add_agent(agent)
    }

    pub fn set_entry_function(&mut self, idx: u32) {
        self.module.entry_function = idx;
    }

    pub fn build(self) -> Module {
        self.module
    }
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_dedup() {
        let mut module = Module::new();
        let idx1 = module.add_constant(Constant::Str("hello".into()));
        let idx2 = module.add_constant(Constant::Str("hello".into()));
        assert_eq!(idx1, idx2);
        assert_eq!(module.constants.len(), 1);
    }

    #[test]
    fn test_module_builder() {
        let mut builder = ModuleBuilder::new();
        let str_idx = builder.add_string_constant("test");
        let num_idx = builder.add_num_constant(42.0);
        assert_eq!(str_idx, 0);
        assert_eq!(num_idx, 1);

        let module = builder.build();
        assert_eq!(module.constants.len(), 2);
    }
}
