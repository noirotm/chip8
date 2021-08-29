use crate::ast::{Addr, Instr, Line, Opcode, VReg};
use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case, take_while, take_while_m_n};
use nom::character::complete::{digit1, hex_digit1, line_ending, not_line_ending, space0, space1};
use nom::combinator::{all_consuming, map, map_res, opt, peek, recognize, verify};
use nom::error::ErrorKind;
use nom::multi::{separated_list0, separated_list1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated, tuple};
use nom::{AsChar, IResult};
use nom::{Finish, InputTakeAtPosition};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::str::FromStr;

fn bin_digit1(i: &str) -> IResult<&str, &str> {
    i.split_at_position1_complete(|c| c != '0' && c != '1', ErrorKind::Digit)
}

fn hex_u16(i: &str) -> IResult<&str, u16> {
    map_res(preceded(tag("0x"), hex_digit1), |s| {
        u16::from_str_radix(s, 16)
    })(i)
}

fn dec_u16(i: &str) -> IResult<&str, u16> {
    map_res(digit1, u16::from_str)(i)
}

fn bin_u16(i: &str) -> IResult<&str, u16> {
    map_res(preceded(tag("0b"), bin_digit1), |s| {
        u16::from_str_radix(s, 2)
    })(i)
}

fn u16(i: &str) -> IResult<&str, u16> {
    alt((hex_u16, bin_u16, dec_u16))(i)
}

fn hex_u8(i: &str) -> IResult<&str, u8> {
    map_res(preceded(tag("0x"), hex_digit1), |s| {
        u8::from_str_radix(s, 16)
    })(i)
}

fn dec_u8(i: &str) -> IResult<&str, u8> {
    map_res(digit1, u8::from_str)(i)
}

fn bin_u8(i: &str) -> IResult<&str, u8> {
    map_res(preceded(tag("0b"), bin_digit1), |s| {
        u8::from_str_radix(s, 2)
    })(i)
}

fn u8(i: &str) -> IResult<&str, u8> {
    alt((hex_u8, bin_u8, dec_u8))(i)
}

fn label(i: &str) -> IResult<&str, &str> {
    recognize(pair(
        take_while_m_n(1, 1, char::is_alphabetic),
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
    ))(i)
}

fn arg_sep(i: &str) -> IResult<&str, &str> {
    delimited(space0, tag(","), space0)(i)
}

fn comment(i: &str) -> IResult<&str, &str> {
    //    recognize(pair(tag("#"), many0(not(line_ending))))(i)
    delimited(tag("#"), not_line_ending, opt(peek(line_ending)))(i)
}

fn data(i: &str) -> IResult<&str, Instr> {
    map(separated_list1(arg_sep, u8), Instr::Data)(i)
}

fn imm_addr(i: &str) -> IResult<&str, Addr> {
    map(u16, Addr::Imm)(i)
}

fn label_ref(i: &str) -> IResult<&str, Addr> {
    map(label, |l| Addr::LabelRef(l.to_owned()))(i)
}

fn addr(i: &str) -> IResult<&str, Addr> {
    alt((imm_addr, label_ref))(i)
}

fn oc_noarg(i: &str) -> IResult<&str, Opcode> {
    let cls = map(tag_no_case("cls"), |_| Opcode::ClearDisplay);
    let ret = map(tag_no_case("ret"), |_| Opcode::Return);

    alt((cls, ret))(i)
}

fn oc_addr(i: &str) -> IResult<&str, Opcode> {
    let jp = map(
        preceded(pair(tag_no_case("jp"), space1), addr),
        Opcode::Jump,
    );
    let call = map(
        preceded(pair(tag_no_case("call"), space1), addr),
        Opcode::Call,
    );
    let ldi = map(
        preceded(
            tuple((tag_no_case("ld"), space1, tag_no_case("i"), arg_sep)),
            addr,
        ),
        Opcode::LoadI,
    );
    let jpv0 = map(
        preceded(
            tuple((tag_no_case("jp"), space1, tag_no_case("v0"), arg_sep)),
            addr,
        ),
        Opcode::JumpV0,
    );

    alt((jp, call, ldi, jpv0))(i)
}

fn vreg(i: &str) -> IResult<&str, VReg> {
    let p = preceded(
        tag_no_case("v"),
        take_while_m_n(1, 1, |b: char| b.is_hex_digit()),
    );
    map_res(p, |s| u8::from_str_radix(s, 16))(i)
}

fn reg_imm(i: &str) -> IResult<&str, (VReg, u8)> {
    preceded(space1, separated_pair(vreg, arg_sep, u8))(i)
}

fn oc_reg_imm(i: &str) -> IResult<&str, Opcode> {
    let se = map(preceded(tag_no_case("se"), reg_imm), |(r, b)| {
        Opcode::SkipEqImm(r, b)
    });
    let sne = map(preceded(tag_no_case("sne"), reg_imm), |(r, b)| {
        Opcode::SkipNotEqImm(r, b)
    });
    let ld = map(preceded(tag_no_case("ld"), reg_imm), |(r, b)| {
        Opcode::LoadImm(r, b)
    });
    let add = map(preceded(tag_no_case("add"), reg_imm), |(r, b)| {
        Opcode::AddImm(r, b)
    });
    let rnd = map(preceded(tag_no_case("rnd"), reg_imm), |(r, b)| {
        Opcode::Random(r, b)
    });

    alt((se, sne, ld, add, rnd))(i)
}

fn reg_reg(i: &str) -> IResult<&str, (VReg, VReg)> {
    preceded(space1, separated_pair(vreg, arg_sep, vreg))(i)
}

fn oc_reg_reg(i: &str) -> IResult<&str, Opcode> {
    let se = map(preceded(tag_no_case("se"), reg_reg), |(r1, r2)| {
        Opcode::SkipEqReg(r1, r2)
    });
    let ld = map(preceded(tag_no_case("ld"), reg_reg), |(r1, r2)| {
        Opcode::LoadReg(r1, r2)
    });
    let or = map(preceded(tag_no_case("or"), reg_reg), |(r1, r2)| {
        Opcode::OrReg(r1, r2)
    });
    let and = map(preceded(tag_no_case("and"), reg_reg), |(r1, r2)| {
        Opcode::AndReg(r1, r2)
    });
    let xor = map(preceded(tag_no_case("xor"), reg_reg), |(r1, r2)| {
        Opcode::XorReg(r1, r2)
    });
    let add = map(preceded(tag_no_case("add"), reg_reg), |(r1, r2)| {
        Opcode::AddReg(r1, r2)
    });
    let sub = map(preceded(tag_no_case("sub"), reg_reg), |(r1, r2)| {
        Opcode::SubReg(r1, r2)
    });
    let shr = map(preceded(tag_no_case("shr"), reg_reg), |(r1, r2)| {
        Opcode::ShiftRight(r1, r2)
    });
    let subn = map(preceded(tag_no_case("subn"), reg_reg), |(r1, r2)| {
        Opcode::SubN(r1, r2)
    });
    let shl = map(preceded(tag_no_case("shl"), reg_reg), |(r1, r2)| {
        Opcode::ShiftLeft(r1, r2)
    });
    let sne = map(preceded(tag_no_case("sne"), reg_reg), |(r1, r2)| {
        Opcode::SkipNotEqReg(r1, r2)
    });

    alt((se, ld, or, and, xor, add, sub, shr, subn, shl, sne))(i)
}

fn oc_reg(i: &str) -> IResult<&str, Opcode> {
    let skp = map(
        preceded(pair(tag_no_case("skp"), space1), vreg),
        Opcode::SkipKeyPressed,
    );
    let skpn = map(
        preceded(pair(tag_no_case("skpn"), space1), vreg),
        Opcode::SkipKeyNotPressed,
    );
    let ld_reg_dt = map(
        delimited(
            pair(tag_no_case("ld"), space1),
            vreg,
            pair(arg_sep, tag_no_case("dt")),
        ),
        Opcode::LoadDelayTimer,
    );
    let ldk = map(
        delimited(
            pair(tag_no_case("ld"), space1),
            vreg,
            pair(arg_sep, tag_no_case("k")),
        ),
        Opcode::WaitKeyPress,
    );
    let ld_dt_reg = map(
        preceded(
            tuple((tag_no_case("ld"), space1, tag_no_case("dt"), arg_sep)),
            vreg,
        ),
        Opcode::SetDelayTimer,
    );
    let ld_st_reg = map(
        preceded(
            tuple((tag_no_case("ld"), space1, tag_no_case("st"), arg_sep)),
            vreg,
        ),
        Opcode::SetSoundTimer,
    );
    let addi = map(
        preceded(
            tuple((tag_no_case("add"), space1, tag_no_case("i"), arg_sep)),
            vreg,
        ),
        Opcode::AddI,
    );
    let ldf = map(
        preceded(
            tuple((tag_no_case("ld"), space1, tag_no_case("f"), arg_sep)),
            vreg,
        ),
        Opcode::LoadSprite,
    );
    let ldb = map(
        preceded(
            tuple((tag_no_case("ld"), space1, tag_no_case("b"), arg_sep)),
            vreg,
        ),
        Opcode::LoadBCD,
    );
    let save_regs = map(
        preceded(
            tuple((tag_no_case("ld"), space1, tag_no_case("[i]"), arg_sep)),
            vreg,
        ),
        Opcode::SaveRegs,
    );
    let ld_regs = map(
        delimited(
            pair(tag_no_case("ld"), space1),
            vreg,
            pair(arg_sep, tag_no_case("[i]")),
        ),
        Opcode::LoadRegs,
    );

    alt((
        skp, skpn, ld_reg_dt, ldk, ld_dt_reg, ld_st_reg, addi, ldf, ldb, save_regs, ld_regs,
    ))(i)
}

fn nibble(i: &str) -> IResult<&str, u8> {
    verify(u8, |&v| v < 16)(i)
}

fn oc_special(i: &str) -> IResult<&str, Opcode> {
    let mut draw = map(
        preceded(tag_no_case("drw"), separated_pair(reg_reg, arg_sep, nibble)),
        |((r1, r2), n)| Opcode::Draw(r1, r2, n),
    );

    draw(i)
}

fn opcode(i: &str) -> IResult<&str, Instr> {
    map(
        alt((
            oc_noarg, oc_addr, oc_reg_imm, oc_reg_reg, oc_reg, oc_special,
        )),
        Instr::Opcode,
    )(i)
}

fn instr(i: &str) -> IResult<&str, Instr> {
    terminated(alt((data, opcode)), space0)(i)
}

fn maybe_label(i: &str) -> IResult<&str, Option<String>> {
    opt(map(terminated(label, tag(":")), String::from))(i)
}

fn maybe_instr(i: &str) -> IResult<&str, Option<Instr>> {
    opt(delimited(space1, instr, space0))(i)
}

fn maybe_comment(i: &str) -> IResult<&str, &str> {
    recognize(opt(comment))(i)
}

fn line(i: &str) -> IResult<&str, Line> {
    map(
        terminated(pair(maybe_label, maybe_instr), maybe_comment),
        |(label, instr)| Line { label, instr },
    )(i)
}

fn lines(i: &str) -> IResult<&str, Vec<Line>> {
    separated_list0(line_ending, line)(i)
}

fn parse_lines(i: &str) -> Result<Vec<Line>, String> {
    all_consuming(lines)(i)
        .finish()
        .map(|(_, l)| l)
        .map_err(|e| e.to_string())
}

pub fn parse_file<P: AsRef<Path>>(p: P) -> Result<Vec<Line>, Box<dyn Error>> {
    let f = File::open(p)?;
    let mut r = BufReader::new(f);
    let mut s = String::new();
    r.read_to_string(&mut s)?;

    parse_lines(&s).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment() {
        let s = "# a comment";
        assert_eq!(comment(s), Ok(("", " a comment")));

        let s = "# a comment\n";
        assert_eq!(comment(s), Ok(("", " a comment")));
    }

    #[test]
    fn test_u16s() {
        let s = "0b0011";
        assert_eq!(bin_u16(s), Ok(("", 3)));

        let s = "0xABCD";
        assert_eq!(hex_u16(s), Ok(("", 0xabcd)));

        let s = "0xabcd";
        assert_eq!(hex_u16(s), Ok(("", 0xabcd)));
    }
}
