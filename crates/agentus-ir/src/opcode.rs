/// Complete instruction set for the Agentus VM.
///
/// Register-based: most instructions operate on register slots within the
/// current call frame. r(A), r(B), r(C) refer to register indices.
///
/// Opcode space uses u8 with deliberate gaps between categories for future expansion.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpCode {
    // =====================================================================
    // CONTROL
    // =====================================================================
    /// No operation.
    Nop = 0x00,
    /// Halt execution.
    Halt = 0x01,

    // =====================================================================
    // LOAD / STORE / MOVE
    // =====================================================================
    /// Load constant: r(A) = constants[Bx]
    LoadConst = 0x10,
    /// Load none: r(A) = none
    LoadNone = 0x11,
    /// Load true: r(A) = true
    LoadTrue = 0x12,
    /// Load false: r(A) = false
    LoadFalse = 0x13,
    /// Copy register: r(A) = r(B)
    Move = 0x14,

    // =====================================================================
    // AGENT MEMORY
    // =====================================================================
    /// Load from agent memory: r(A) = agent_memory[constants[Bx]]
    MLoad = 0x20,
    /// Store to agent memory: agent_memory[constants[Bx]] = r(A)
    MStore = 0x21,
    /// Load from global memory: r(A) = global_memory[constants[Bx]]
    GLoad = 0x22,
    /// Store to global memory: global_memory[constants[Bx]] = r(A)
    GStore = 0x23,

    // =====================================================================
    // ARITHMETIC
    // =====================================================================
    /// Add: r(A) = r(B) + r(C)
    Add = 0x30,
    /// Subtract: r(A) = r(B) - r(C)
    Sub = 0x31,
    /// Multiply: r(A) = r(B) * r(C)
    Mul = 0x32,
    /// Divide: r(A) = r(B) / r(C)
    Div = 0x33,
    /// Modulo: r(A) = r(B) % r(C)
    Mod = 0x34,
    /// Negate: r(A) = -r(B)
    Neg = 0x35,

    // =====================================================================
    // COMPARISON
    // =====================================================================
    /// Equal: r(A) = r(B) == r(C)
    Eq = 0x40,
    /// Not equal: r(A) = r(B) != r(C)
    Neq = 0x41,
    /// Less than: r(A) = r(B) < r(C)
    Lt = 0x42,
    /// Less than or equal: r(A) = r(B) <= r(C)
    Lte = 0x43,
    /// Greater than: r(A) = r(B) > r(C)
    Gt = 0x44,
    /// Greater than or equal: r(A) = r(B) >= r(C)
    Gte = 0x45,

    // =====================================================================
    // LOGIC
    // =====================================================================
    /// Logical AND: r(A) = r(B) && r(C)
    And = 0x48,
    /// Logical OR: r(A) = r(B) || r(C)
    Or = 0x49,
    /// Logical NOT: r(A) = !r(B)
    Not = 0x4A,

    // =====================================================================
    // STRING OPERATIONS
    // =====================================================================
    /// Concatenate: r(A) = str(r(B)) ++ str(r(C))
    Concat = 0x50,
    /// String length: r(A) = len(r(B))
    StrLen = 0x51,
    /// Format template: r(A) = format(constants[Bx], r(C)..r(C+N))
    Format = 0x52,
    /// Substring: r(A) = r(B)\[r(C)..r(D)\]
    Substr = 0x53,

    // =====================================================================
    // COLLECTION OPERATIONS
    // =====================================================================
    /// Create list from consecutive registers: r(A) = list(r(B)..r(B+C))
    NewList = 0x58,
    /// Create map from consecutive key-value pairs: r(A) = map(r(B)..r(B+C*2))
    NewMap = 0x59,
    /// Index get: r(A) = r(B)\[r(C)\]
    IndexGet = 0x5A,
    /// Index set: r(A)\[r(B)\] = r(C)
    IndexSet = 0x5B,
    /// Length: r(A) = len(r(B))
    Len = 0x5C,
    /// Push to list: r(A).push(r(B))
    ListPush = 0x5D,

    // =====================================================================
    // CONTROL FLOW
    // =====================================================================
    /// Unconditional jump: PC += sBx
    Jmp = 0x60,
    /// Jump if truthy: if r(A) then PC += sBx
    JmpTrue = 0x61,
    /// Jump if falsy: if !r(A) then PC += sBx
    JmpFalse = 0x62,

    // =====================================================================
    // FUNCTION CALL / RETURN
    // =====================================================================
    /// Call function: r(A) = call(func_table[Bx], r(C)..r(C+N))
    Call = 0x68,
    /// Return value: return r(A)
    Ret = 0x69,
    /// Return none
    RetNone = 0x6A,

    // =====================================================================
    // LLM EXECUTION
    // =====================================================================
    /// Execute LLM prompt: r(A) = exec(prompt=r(B))
    Exec = 0x70,
    /// Execute with structured output: r(A) = exec(prompt=r(B), schema=r(C))
    ExecStructured = 0x71,

    // =====================================================================
    // AGENT OPERATIONS
    // =====================================================================
    /// Spawn agent: r(A) = spawn(agent_type=constants[Bx], init=r(C)..r(C+N))
    Spawn = 0x78,
    /// Send message: send(handle=r(A), message=r(B))
    Send = 0x79,
    /// Receive message (blocking): r(A) = recv(handle=r(B))
    Recv = 0x7A,
    /// Receive with timeout: r(A) = recv(handle=r(B), timeout_ms=r(C))
    RecvTimeout = 0x7B,
    /// Wait for agent completion: r(A) = wait(handle=r(B))
    Wait = 0x7C,
    /// Kill agent: kill(handle=r(A))
    Kill = 0x7D,

    // =====================================================================
    // TOOL INVOCATION
    // =====================================================================
    /// Call tool: r(A) = tcall(tool=constants[Bx], r(C)..r(C+N))
    TCall = 0x80,

    // =====================================================================
    // PIPELINE
    // =====================================================================
    /// Run pipeline: r(A) = pipeline_run(pipeline=constants[Bx], input=r(C))
    PipelineRun = 0x88,

    // =====================================================================
    // I/O
    // =====================================================================
    /// Emit output: emit(r(A))
    Emit = 0x90,
    /// Log message: log(level=B, message=r(C))
    Log = 0x91,

    // =====================================================================
    // ERROR HANDLING
    // =====================================================================
    /// Begin try block: push error handler at PC + sBx
    TryBegin = 0x98,
    /// End try block: pop error handler
    TryEnd = 0x99,
    /// Throw error: throw(r(A))
    Throw = 0x9A,
    /// Get current error: r(A) = current_error
    GetError = 0x9B,

    // =====================================================================
    // COROUTINE
    // =====================================================================
    /// Yield execution: yield(r(A))
    Yield = 0xA0,

    // =====================================================================
    // ITERATOR
    // =====================================================================
    /// Create iterator: r(A) = iter(r(B))
    IterInit = 0xA8,
    /// Advance iterator: r(A) = next(r(B)), jump sBx if exhausted
    IterNext = 0xA9,

    // =====================================================================
    // TYPE OPERATIONS
    // =====================================================================
    /// Type of: r(A) = typeof(r(B))
    TypeOf = 0xB0,
    /// Cast: r(A) = cast(r(B), type=C)
    Cast = 0xB1,
}

impl OpCode {
    /// Decode a u8 into an OpCode, returning None for invalid values.
    pub fn from_byte(byte: u8) -> Option<Self> {
        // Safety: we validate the byte is a known opcode
        match byte {
            0x00 => Some(Self::Nop),
            0x01 => Some(Self::Halt),

            0x10 => Some(Self::LoadConst),
            0x11 => Some(Self::LoadNone),
            0x12 => Some(Self::LoadTrue),
            0x13 => Some(Self::LoadFalse),
            0x14 => Some(Self::Move),

            0x20 => Some(Self::MLoad),
            0x21 => Some(Self::MStore),
            0x22 => Some(Self::GLoad),
            0x23 => Some(Self::GStore),

            0x30 => Some(Self::Add),
            0x31 => Some(Self::Sub),
            0x32 => Some(Self::Mul),
            0x33 => Some(Self::Div),
            0x34 => Some(Self::Mod),
            0x35 => Some(Self::Neg),

            0x40 => Some(Self::Eq),
            0x41 => Some(Self::Neq),
            0x42 => Some(Self::Lt),
            0x43 => Some(Self::Lte),
            0x44 => Some(Self::Gt),
            0x45 => Some(Self::Gte),

            0x48 => Some(Self::And),
            0x49 => Some(Self::Or),
            0x4A => Some(Self::Not),

            0x50 => Some(Self::Concat),
            0x51 => Some(Self::StrLen),
            0x52 => Some(Self::Format),
            0x53 => Some(Self::Substr),

            0x58 => Some(Self::NewList),
            0x59 => Some(Self::NewMap),
            0x5A => Some(Self::IndexGet),
            0x5B => Some(Self::IndexSet),
            0x5C => Some(Self::Len),
            0x5D => Some(Self::ListPush),

            0x60 => Some(Self::Jmp),
            0x61 => Some(Self::JmpTrue),
            0x62 => Some(Self::JmpFalse),

            0x68 => Some(Self::Call),
            0x69 => Some(Self::Ret),
            0x6A => Some(Self::RetNone),

            0x70 => Some(Self::Exec),
            0x71 => Some(Self::ExecStructured),

            0x78 => Some(Self::Spawn),
            0x79 => Some(Self::Send),
            0x7A => Some(Self::Recv),
            0x7B => Some(Self::RecvTimeout),
            0x7C => Some(Self::Wait),
            0x7D => Some(Self::Kill),

            0x80 => Some(Self::TCall),

            0x88 => Some(Self::PipelineRun),

            0x90 => Some(Self::Emit),
            0x91 => Some(Self::Log),

            0x98 => Some(Self::TryBegin),
            0x99 => Some(Self::TryEnd),
            0x9A => Some(Self::Throw),
            0x9B => Some(Self::GetError),

            0xA0 => Some(Self::Yield),

            0xA8 => Some(Self::IterInit),
            0xA9 => Some(Self::IterNext),

            0xB0 => Some(Self::TypeOf),
            0xB1 => Some(Self::Cast),

            _ => None,
        }
    }

    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
