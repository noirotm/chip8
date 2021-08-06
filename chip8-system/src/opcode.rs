use crate::system::VReg;
use num_traits::FromPrimitive;

#[derive(Debug, PartialEq)]
pub(crate) enum Instr {
    ClearDisplay,
    Return,
    Jump(u16),
    Call(u16),
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
    LoadI(u16),
    JumpV0(u16),
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

fn nnn(opcode: u16) -> u16 {
    opcode & 0xFFF
}

fn x(opcode: u16) -> VReg {
    VReg::from_u16((opcode >> 8) & 0x0F).unwrap()
}

fn y(opcode: u16) -> VReg {
    VReg::from_u16((opcode >> 4) & 0x0F).unwrap()
}

fn kk(opcode: u16) -> u8 {
    (opcode & 0xFF) as u8
}

fn n(opcode: u16) -> u8 {
    (opcode & 0xF) as u8
}

pub(crate) fn parse_opcode(opcode: u16) -> Option<Instr> {
    let msn = opcode >> 12;
    let lsn = opcode & 0xF;
    match opcode {
        0x00E0 => Some(Instr::ClearDisplay),
        0x00EE => Some(Instr::Return),
        o if msn == 0x1 => Some(Instr::Jump(nnn(o))),
        o if msn == 0x2 => Some(Instr::Call(nnn(o))),
        o if msn == 0x3 => Some(Instr::SkipEqImm(x(o), kk(o))),
        o if msn == 0x4 => Some(Instr::SkipNotEqImm(x(o), kk(o))),
        o if (msn, lsn) == (0x5, 0x0) => Some(Instr::SkipEqReg(x(o), y(o))),
        o if msn == 0x6 => Some(Instr::LoadImm(x(o), kk(o))),
        o if msn == 0x7 => Some(Instr::AddImm(x(o), kk(o))),
        o if msn == 0x8 => parse_opcode_8(o),
        o if (msn, lsn) == (0x9, 0x0) => Some(Instr::SkipNotEqReg(x(o), y(o))),
        o if msn == 0xA => Some(Instr::LoadI(nnn(o))),
        o if msn == 0xB => Some(Instr::JumpV0(nnn(o))),
        o if msn == 0xC => Some(Instr::Random(x(o), kk(o))),
        o if msn == 0xD => Some(Instr::Draw(x(o), y(o), n(o))),
        o if msn == 0xE => parse_opcode_e(o),
        o if msn == 0xF => parse_opcode_f(o),
        _ => None,
    }
}

fn parse_opcode_8(opcode: u16) -> Option<Instr> {
    let lsn = opcode & 0xF;
    let (x, y) = (x(opcode), y(opcode));
    match lsn {
        0x0 => Some(Instr::LoadReg(x, y)),
        0x1 => Some(Instr::OrReg(x, y)),
        0x2 => Some(Instr::AndReg(x, y)),
        0x3 => Some(Instr::XorReg(x, y)),
        0x4 => Some(Instr::AddReg(x, y)),
        0x5 => Some(Instr::SubReg(x, y)),
        0x6 => Some(Instr::ShiftRight(x, y)),
        0x7 => Some(Instr::SubN(x, y)),
        0xE => Some(Instr::ShiftLeft(x, y)),
        _ => None,
    }
}

fn parse_opcode_e(opcode: u16) -> Option<Instr> {
    let lsb = opcode & 0xFF;
    let x = x(opcode);
    match lsb {
        0x9E => Some(Instr::SkipKeyPressed(x)),
        0xA1 => Some(Instr::SkipKeyNotPressed(x)),
        _ => None,
    }
}

fn parse_opcode_f(opcode: u16) -> Option<Instr> {
    let lsb = opcode & 0xFF;
    let x = x(opcode);
    match lsb {
        0x07 => Some(Instr::LoadDelayTimer(x)),
        0x0A => Some(Instr::WaitKeyPress(x)),
        0x15 => Some(Instr::SetDelayTimer(x)),
        0x18 => Some(Instr::SetSoundTimer(x)),
        0x1E => Some(Instr::AddI(x)),
        0x29 => Some(Instr::LoadSprite(x)),
        0x33 => Some(Instr::LoadBCD(x)),
        0x55 => Some(Instr::SaveRegs(x)),
        0x65 => Some(Instr::LoadRegs(x)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::Instr::*;
    use crate::system::VReg::*;

    #[test]
    fn test_parse_opcode() {
        let test_cases = [
            (0x00E0, ClearDisplay),
            (0x00EE, Return),
            (0x1123, Jump(0x123)),
            (0x2123, Call(0x123)),
            (0x3136, SkipEqImm(V1, 0x36)),
            (0x4A36, SkipNotEqImm(VA, 0x36)),
            (0x5560, SkipEqReg(V5, V6)),
            (0x6247, LoadImm(V2, 0x47)),
            (0x7A71, AddImm(VA, 0x71)),
            (0x85A0, LoadReg(V5, VA)),
            (0x85A1, OrReg(V5, VA)),
            (0x85A2, AndReg(V5, VA)),
            (0x85A3, XorReg(V5, VA)),
            (0x85A4, AddReg(V5, VA)),
            (0x85A5, SubReg(V5, VA)),
            (0x85A6, ShiftRight(V5, VA)),
            (0x85A7, SubN(V5, VA)),
            (0x85AE, ShiftLeft(V5, VA)),
            (0x9470, SkipNotEqReg(V4, V7)),
            (0xA123, LoadI(0x123)),
            (0xB72F, JumpV0(0x72F)),
            (0xCA48, Random(VA, 0x48)),
            (0xD737, Draw(V7, V3, 0x7)),
            (0xE59E, SkipKeyPressed(V5)),
            (0xE5A1, SkipKeyNotPressed(V5)),
            (0xF207, LoadDelayTimer(V2)),
            (0xF20A, WaitKeyPress(V2)),
            (0xF215, SetDelayTimer(V2)),
            (0xF218, SetSoundTimer(V2)),
            (0xF21E, AddI(V2)),
            (0xF229, LoadSprite(V2)),
            (0xF233, LoadBCD(V2)),
            (0xF255, SaveRegs(V2)),
            (0xF265, LoadRegs(V2)),
        ];

        for (o, i) in test_cases {
            assert_eq!(parse_opcode(o), Some(i));
        }
    }

    #[test]
    fn test_parse_bad_opcode() {
        let test_cases = [0x0000, 0x5561, 0x8458, 0x9127, 0xE501, 0xF501];

        for o in test_cases {
            assert!(parse_opcode(o).is_none());
        }
    }
}
