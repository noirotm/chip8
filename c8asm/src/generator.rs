use crate::ast::{Addr, Instr, Line, Opcode, VReg};
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;

const ORIGIN: usize = 0x200;

pub fn generate<W: Write>(lines: &[Line], w: &mut W) -> Result<(), Box<dyn Error>> {
    let labels = labels(lines)?;
    opcodes(lines, &labels, w)
}

fn labels(lines: &[Line]) -> Result<HashMap<String, usize>, String> {
    let mut addr = ORIGIN;
    let mut labels = HashMap::new();

    for l in lines {
        if let Some(label) = &l.label {
            // forbid duplicate labels
            if labels.contains_key(label) {
                return Err(format!("duplicate label: '{}'", label));
            }
            labels.insert(label.to_owned(), addr);
        }
        addr += l.size();
    }

    Ok(labels)
}

fn opcodes<W: Write>(
    lines: &[Line],
    labels: &HashMap<String, usize>,
    w: &mut W,
) -> Result<(), Box<dyn Error>> {
    for line in lines {
        match &line.instr {
            Some(Instr::Data(d)) => w.write_all(d)?,
            Some(Instr::Opcode(o)) => opcode(o, labels, w)?,
            _ => {}
        }
    }

    Ok(())
}

fn opcode<W: Write>(
    o: &Opcode,
    labels: &HashMap<String, usize>,
    w: &mut W,
) -> Result<(), Box<dyn Error>> {
    let code = match o {
        Opcode::ClearDisplay => 0x00E0,
        Opcode::Return => 0x00EE,
        Opcode::Jump(a) => addr(0x1000, a, labels)?,
        Opcode::Call(a) => addr(0x2000, a, labels)?,
        Opcode::SkipEqImm(r, b) => reg_imm(0x3000, *r, *b),
        Opcode::SkipNotEqImm(r, b) => reg_imm(0x4000, *r, *b),
        Opcode::SkipEqReg(r1, r2) => reg_reg(0x5000, *r1, *r2),
        Opcode::LoadImm(r, b) => reg_imm(0x6000, *r, *b),
        Opcode::AddImm(r, b) => reg_imm(0x7000, *r, *b),
        Opcode::LoadReg(r1, r2) => reg_reg(0x8000, *r1, *r2),
        Opcode::OrReg(r1, r2) => reg_reg(0x8001, *r1, *r2),
        Opcode::AndReg(r1, r2) => reg_reg(0x8002, *r1, *r2),
        Opcode::XorReg(r1, r2) => reg_reg(0x8003, *r1, *r2),
        Opcode::AddReg(r1, r2) => reg_reg(0x8004, *r1, *r2),
        Opcode::SubReg(r1, r2) => reg_reg(0x8005, *r1, *r2),
        Opcode::ShiftRight(r1, r2) => reg_reg(0x8006, *r1, *r2),
        Opcode::SubN(r1, r2) => reg_reg(0x8007, *r1, *r2),
        Opcode::ShiftLeft(r1, r2) => reg_reg(0x800E, *r1, *r2),
        Opcode::SkipNotEqReg(r1, r2) => reg_reg(0x9000, *r1, *r2),
        Opcode::LoadI(a) => addr(0xA000, a, labels)?,
        Opcode::JumpV0(a) => addr(0xB000, a, labels)?,
        Opcode::Random(r, b) => reg_imm(0xC000, *r, *b),
        Opcode::Draw(r1, r2, n) => reg_reg_nib(0xD000, *r1, *r2, *n),
        Opcode::SkipKeyPressed(r) => reg(0xE09E, *r),
        Opcode::SkipKeyNotPressed(r) => reg(0xE0A1, *r),
        Opcode::LoadDelayTimer(r) => reg(0xF007, *r),
        Opcode::WaitKeyPress(r) => reg(0xF00A, *r),
        Opcode::SetDelayTimer(r) => reg(0xF015, *r),
        Opcode::SetSoundTimer(r) => reg(0xF018, *r),
        Opcode::AddI(r) => reg(0xF01E, *r),
        Opcode::LoadSprite(r) => reg(0xF029, *r),
        Opcode::LoadBCD(r) => reg(0xF033, *r),
        Opcode::SaveRegs(r) => reg(0xF055, *r),
        Opcode::LoadRegs(r) => reg(0xF065, *r),
    };

    w.write_all(&code.to_be_bytes()).map_err(|e| e.into())
}

fn addr(c: u16, addr: &Addr, labels: &HashMap<String, usize>) -> Result<u16, String> {
    let a = match addr {
        Addr::Imm(a) => *a as usize,
        Addr::LabelRef(s) => *labels.get(s).ok_or(format!("unknown label: '{}'", s))?,
    };
    Ok(c | (a & 0xFFF) as u16)
}

fn reg_imm(c: u16, r: VReg, b: u8) -> u16 {
    c | ((r as u16) << 8) | (b as u16)
}

fn reg_reg(c: u16, r1: VReg, r2: VReg) -> u16 {
    c | ((r1 as u16) << 8) | ((r2 as u16) << 4)
}

fn reg_reg_nib(c: u16, r1: VReg, r2: VReg, n: u8) -> u16 {
    c | ((r1 as u16) << 8) | ((r2 as u16) << 4) | (n as u16 & 0xF)
}

fn reg(c: u16, r: VReg) -> u16 {
    c | ((r as u16) << 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addr() {
        assert_eq!(
            addr(0x1000, &Addr::Imm(0x251), &Default::default()),
            Ok(0x1251)
        );
    }

    #[test]
    fn test_reg_imm() {
        assert_eq!(reg_imm(0x5000, 2, 0xAB), 0x52AB);
    }

    #[test]
    fn test_reg_reg() {
        assert_eq!(reg_reg(0x5000, 4, 8), 0x5480);
        assert_eq!(reg_reg(0x5005, 4, 8), 0x5485);
    }

    #[test]
    fn test_reg_reg_nib() {
        assert_eq!(reg_reg_nib(0xD000, 4, 8, 3), 0xD483);
    }

    #[test]
    fn test_reg() {
        assert_eq!(reg(0x5000, 4), 0x5400);
        assert_eq!(reg(0x5005, 8), 0x5805);
    }
}
