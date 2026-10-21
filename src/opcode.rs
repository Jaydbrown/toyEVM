#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    Stop = 0x00,
    Add = 0x01,
    Mul = 0x02,
    Sub = 0x03,
    Div = 0x04,
    Mod = 0x06,
    Lt = 0x10,
    Gt = 0x11,
    Eq = 0x14,
    IsZero = 0x15,
    And = 0x16,
    Or = 0x17,
    Xor = 0x18,
    Not = 0x19,
    Keccak256 = 0x20,
    CallValue = 0x34,
    CallDataLoad = 0x35,
    CallDataSize = 0x36,
    Pop = 0x50,
    MLoad = 0x51,
    MStore = 0x52,
    MStore8 = 0x53,
    SLoad = 0x54,
    SStore = 0x55,
    Jump = 0x56,
    JumpI = 0x57,
    Pc = 0x58,
    MSize = 0x59,
    Gas = 0x5a,
    JumpDest = 0x5b,
    Push0 = 0x5f,
    Push1 = 0x60,
    Push2 = 0x61,
    Push4 = 0x63,
    Push8 = 0x67,
    Push16 = 0x6f,
    Push20 = 0x73,
    Push32 = 0x7f,
    Dup1 = 0x80,
    Dup2 = 0x81,
    Swap1 = 0x90,
    Swap2 = 0x91,
    Return = 0xf3,
    Revert = 0xfd,
    Invalid = 0xfe,
}

impl Opcode {
    pub fn gas_cost(self) -> u64 {
        match self {
            Opcode::Stop | Opcode::Return | Opcode::Revert | Opcode::Invalid => 0,
            Opcode::JumpDest => 1,
            Opcode::CallValue
            | Opcode::CallDataSize
            | Opcode::Pop
            | Opcode::Pc
            | Opcode::MSize
            | Opcode::Gas
            | Opcode::Push0 => 2,
            Opcode::Add
            | Opcode::Sub
            | Opcode::Lt
            | Opcode::Gt
            | Opcode::Eq
            | Opcode::IsZero
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Not
            | Opcode::CallDataLoad
            | Opcode::MLoad
            | Opcode::MStore
            | Opcode::MStore8
            | Opcode::Push1
            | Opcode::Push2
            | Opcode::Push4
            | Opcode::Push8
            | Opcode::Push16
            | Opcode::Push20
            | Opcode::Push32
            | Opcode::Dup1
            | Opcode::Dup2
            | Opcode::Swap1
            | Opcode::Swap2 => 3,
            Opcode::Mul | Opcode::Div | Opcode::Mod => 5,
            Opcode::Jump => 8,
            Opcode::JumpI => 10,
            Opcode::Keccak256 => 30,
            Opcode::SLoad => 100,
            Opcode::SStore => 2000,
        }
    }
}

impl TryFrom<u8> for Opcode {
    type Error = &'static str;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Opcode::Stop),
            0x01 => Ok(Opcode::Add),
            0x02 => Ok(Opcode::Mul),
            0x03 => Ok(Opcode::Sub),
            0x04 => Ok(Opcode::Div),
            0x06 => Ok(Opcode::Mod),
            0x10 => Ok(Opcode::Lt),
            0x11 => Ok(Opcode::Gt),
            0x14 => Ok(Opcode::Eq),
            0x15 => Ok(Opcode::IsZero),
            0x16 => Ok(Opcode::And),
            0x17 => Ok(Opcode::Or),
            0x18 => Ok(Opcode::Xor),
            0x19 => Ok(Opcode::Not),
            0x20 => Ok(Opcode::Keccak256),
            0x34 => Ok(Opcode::CallValue),
            0x35 => Ok(Opcode::CallDataLoad),
            0x36 => Ok(Opcode::CallDataSize),
            0x50 => Ok(Opcode::Pop),
            0x51 => Ok(Opcode::MLoad),
            0x52 => Ok(Opcode::MStore),
            0x53 => Ok(Opcode::MStore8),
            0x54 => Ok(Opcode::SLoad),
            0x55 => Ok(Opcode::SStore),
            0x56 => Ok(Opcode::Jump),
            0x57 => Ok(Opcode::JumpI),
            0x58 => Ok(Opcode::Pc),
            0x59 => Ok(Opcode::MSize),
            0x5a => Ok(Opcode::Gas),
            0x5b => Ok(Opcode::JumpDest),
            0x5f => Ok(Opcode::Push0),
            0x60 => Ok(Opcode::Push1),
            0x61 => Ok(Opcode::Push2),
            0x63 => Ok(Opcode::Push4),
            0x67 => Ok(Opcode::Push8),
            0x6f => Ok(Opcode::Push16),
            0x73 => Ok(Opcode::Push20),
            0x7f => Ok(Opcode::Push32),
            0x80 => Ok(Opcode::Dup1),
            0x81 => Ok(Opcode::Dup2),
            0x90 => Ok(Opcode::Swap1),
            0x91 => Ok(Opcode::Swap2),
            0xf3 => Ok(Opcode::Return),
            0xfd => Ok(Opcode::Revert),
            0xfe => Ok(Opcode::Invalid),
            _ => Err("Unknown or unsupported EVM opcode"),
        }
    }
}
