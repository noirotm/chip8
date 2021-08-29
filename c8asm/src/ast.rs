#[derive(Debug)]
pub struct Line {
    pub label: Option<String>,
    pub instr: Option<Instr>,
}

impl Line {
    pub fn size(&self) -> usize {
        match &self.instr {
            Some(i) => i.size(),
            None => 0,
        }
    }
}

#[derive(Debug)]
pub enum Instr {
    Opcode(Opcode),
    Data(Vec<u8>),
}

impl Instr {
    pub fn size(&self) -> usize {
        match self {
            Instr::Opcode(_) => 2,
            Instr::Data(d) => d.len(),
        }
    }
}

#[derive(Debug)]
pub enum Opcode {
    ClearDisplay,
    Return,
    Jump(Addr),
    Call(Addr),
    SkipEqImm(VReg, u8),
    SkipNotEqImm(VReg, u8),
    SkipEqReg(VReg, VReg),
    LoadImm(VReg, u8),
    AddImm(VReg, u8),
    LoadReg(VReg, VReg),
    OrReg(VReg, VReg),
    AndReg(VReg, VReg),
    XorReg(VReg, VReg),
    AddReg(VReg, VReg),
    SubReg(VReg, VReg),
    ShiftRight(VReg, VReg),
    SubN(VReg, VReg),
    ShiftLeft(VReg, VReg),
    SkipNotEqReg(VReg, VReg),
    LoadI(Addr),
    JumpV0(Addr),
    Random(VReg, u8),
    Draw(VReg, VReg, u8),
    SkipKeyPressed(VReg),
    SkipKeyNotPressed(VReg),
    LoadDelayTimer(VReg),
    WaitKeyPress(VReg),
    SetDelayTimer(VReg),
    SetSoundTimer(VReg),
    AddI(VReg),
    LoadSprite(VReg),
    LoadBCD(VReg),
    SaveRegs(VReg),
    LoadRegs(VReg),
}

pub type VReg = u8;

#[derive(Debug)]
pub enum Addr {
    Imm(u16),
    LabelRef(String),
}
