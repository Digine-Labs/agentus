use crate::opcode::OpCode;

/// A 32-bit encoded instruction.
///
/// Three encoding formats:
/// - ABC:  opcode(8) | A(8) | B(8) | C(8)    — three register operands
/// - ABx:  opcode(8) | A(8) | Bx(16)          — register + unsigned 16-bit constant index
/// - AsBx: opcode(8) | A(8) | sBx(16 signed)  — register + signed 16-bit offset
/// - sBx:  opcode(8) | sBx(24 signed)          — signed 24-bit offset (no register)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction(pub u32);

impl Instruction {
    // =====================================================================
    // Constructors
    // =====================================================================

    /// Encode an ABC-format instruction.
    pub fn abc(op: OpCode, a: u8, b: u8, c: u8) -> Self {
        let word = (op.to_byte() as u32) << 24
            | (a as u32) << 16
            | (b as u32) << 8
            | (c as u32);
        Self(word)
    }

    /// Encode an ABx-format instruction (register + 16-bit unsigned index).
    pub fn abx(op: OpCode, a: u8, bx: u16) -> Self {
        let word = (op.to_byte() as u32) << 24
            | (a as u32) << 16
            | (bx as u32);
        Self(word)
    }

    /// Encode an AsBx-format instruction (register + 16-bit signed offset).
    pub fn asbx(op: OpCode, a: u8, sbx: i16) -> Self {
        let word = (op.to_byte() as u32) << 24
            | (a as u32) << 16
            | (sbx as u16 as u32);
        Self(word)
    }

    /// Encode an sBx-format instruction (24-bit signed offset, no register).
    pub fn sbx(op: OpCode, offset: i32) -> Self {
        // Encode as 24-bit signed: mask to 24 bits
        let masked = (offset as u32) & 0x00FF_FFFF;
        let word = (op.to_byte() as u32) << 24 | masked;
        Self(word)
    }

    /// Encode an instruction with only opcode (no operands).
    pub fn op_only(op: OpCode) -> Self {
        Self((op.to_byte() as u32) << 24)
    }

    /// Encode an instruction with opcode + single register A.
    pub fn op_a(op: OpCode, a: u8) -> Self {
        Self((op.to_byte() as u32) << 24 | (a as u32) << 16)
    }

    // =====================================================================
    // Decoders
    // =====================================================================

    /// Extract the opcode byte.
    pub fn opcode_byte(&self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// Decode the opcode.
    pub fn opcode(&self) -> Option<OpCode> {
        OpCode::from_byte(self.opcode_byte())
    }

    /// Extract register A (bits 23..16).
    pub fn a(&self) -> u8 {
        (self.0 >> 16) as u8
    }

    /// Extract register B (bits 15..8).
    pub fn b(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    /// Extract register C (bits 7..0).
    pub fn c(&self) -> u8 {
        self.0 as u8
    }

    /// Extract unsigned 16-bit Bx (bits 15..0).
    pub fn bx(&self) -> u16 {
        self.0 as u16
    }

    /// Extract signed 16-bit sBx (bits 15..0).
    pub fn sbx_16(&self) -> i16 {
        self.0 as u16 as i16
    }

    /// Extract signed 24-bit sBx (bits 23..0).
    pub fn sbx_24(&self) -> i32 {
        let raw = self.0 & 0x00FF_FFFF;
        // Sign-extend from 24 bits
        if raw & 0x0080_0000 != 0 {
            (raw | 0xFF00_0000) as i32
        } else {
            raw as i32
        }
    }

    /// Get the raw 32-bit word.
    pub fn raw(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.opcode() {
            Some(op) => write!(
                f,
                "{:<16} A={:<3} B={:<3} C={:<3} Bx={:<5}",
                op,
                self.a(),
                self.b(),
                self.c(),
                self.bx()
            ),
            None => write!(f, "UNKNOWN(0x{:02X})", self.opcode_byte()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::OpCode;

    #[test]
    fn test_abc_roundtrip() {
        let inst = Instruction::abc(OpCode::Add, 3, 1, 2);
        assert_eq!(inst.opcode(), Some(OpCode::Add));
        assert_eq!(inst.a(), 3);
        assert_eq!(inst.b(), 1);
        assert_eq!(inst.c(), 2);
    }

    #[test]
    fn test_abx_roundtrip() {
        let inst = Instruction::abx(OpCode::LoadConst, 5, 1000);
        assert_eq!(inst.opcode(), Some(OpCode::LoadConst));
        assert_eq!(inst.a(), 5);
        assert_eq!(inst.bx(), 1000);
    }

    #[test]
    fn test_sbx_24_positive() {
        let inst = Instruction::sbx(OpCode::Jmp, 42);
        assert_eq!(inst.opcode(), Some(OpCode::Jmp));
        assert_eq!(inst.sbx_24(), 42);
    }

    #[test]
    fn test_sbx_24_negative() {
        let inst = Instruction::sbx(OpCode::Jmp, -10);
        assert_eq!(inst.opcode(), Some(OpCode::Jmp));
        assert_eq!(inst.sbx_24(), -10);
    }

    #[test]
    fn test_op_only() {
        let inst = Instruction::op_only(OpCode::Halt);
        assert_eq!(inst.opcode(), Some(OpCode::Halt));
        assert_eq!(inst.a(), 0);
    }

    #[test]
    fn test_emit() {
        let inst = Instruction::op_a(OpCode::Emit, 7);
        assert_eq!(inst.opcode(), Some(OpCode::Emit));
        assert_eq!(inst.a(), 7);
    }
}
