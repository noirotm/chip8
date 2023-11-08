use crate::display::{font_sprites, DisplayBuffer, FONT_SPRITES_ADDRESS};
use crate::keyboard::{Key, Keyboard, KeyboardController};
use crate::memory::{Memory, RESERVED_SIZE};
use crate::opcode::{parse_opcode, Instr};
use crate::port::ControlPin;
use crate::timer::{CountDownTimer, ObservableTimer};
use bitflags::bitflags;
use num_derive::FromPrimitive;
use rand::prelude::SmallRng;
use rand::{Rng, SeedableRng};
use spin_sleep::LoopHelper;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::ops::{Index, IndexMut};
use std::path::Path;
use std::thread::JoinHandle;
use std::{io, thread};

#[derive(Debug)]
pub enum SystemError {
    OddPcAddress,
    UnknownInstruction,
    MemoryReadOverflow,
    StackUnderflow,
    StackOverflow,
    SelfJump,
    Interrupted,
}

impl Display for SystemError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for SystemError {}

bitflags! {
    pub struct Quirks: u8 {
        const LOAD_STORE_IGNORES_I = 0x1;
        const SHIFT_READS_VX = 0x2;
        const DRAW_WRAPS_PIXELS = 0x4;
    }
}

#[derive(Copy, Clone, Debug, FromPrimitive, PartialEq)]
pub(crate) enum VReg {
    V0 = 0x0,
    V1 = 0x1,
    V2 = 0x2,
    V3 = 0x3,
    V4 = 0x4,
    V5 = 0x5,
    V6 = 0x6,
    V7 = 0x7,
    V8 = 0x8,
    V9 = 0x9,
    VA = 0xa,
    VB = 0xb,
    VC = 0xc,
    VD = 0xd,
    VE = 0xe,
    VF = 0xf,
}

type VRegBank = [u8; 16];

impl Index<VReg> for VRegBank {
    type Output = u8;

    fn index(&self, index: VReg) -> &Self::Output {
        let idx = index as usize;
        self.index(idx)
    }
}

impl IndexMut<VReg> for VRegBank {
    fn index_mut(&mut self, index: VReg) -> &mut Self::Output {
        let idx = index as usize;
        self.index_mut(idx)
    }
}

const STACK_SIZE: usize = 16;

struct Cpu {
    pc: u16,
    v: VRegBank,
    i: u16,
    stack: Vec<u16>,
}

pub struct SystemOptions {
    cpu_frequency_hz: f64,
    quirks: Quirks,
}

impl Default for SystemOptions {
    fn default() -> Self {
        Self {
            cpu_frequency_hz: 500.0,
            quirks: Quirks::empty(),
        }
    }
}

impl SystemOptions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn cpu_frequency_hz(&mut self, f: f64) -> &mut Self {
        self.cpu_frequency_hz = if f < 5000.0 && f > 0.0 { f } else { 500.0 };
        self
    }

    pub fn quirk(&mut self, quirk: Quirks) -> &mut Self {
        self.quirks |= quirk;
        self
    }
}

pub struct SystemController {
    stop_pin: ControlPin,
    kb_controller: KeyboardController,
}

impl SystemController {
    pub fn stop(&self) {
        self.stop_pin.raise();
        self.kb_controller.stop();
    }
}

pub struct System {
    cpu: Cpu,
    delay_timer: CountDownTimer,
    pub sound_timer: CountDownTimer,
    pub keyboard: Keyboard,
    pub display: DisplayBuffer,
    stop: ControlPin,
    memory: Memory,
    options: SystemOptions,
}

impl Default for System {
    fn default() -> Self {
        Self::new_with_options(Default::default())
    }
}

impl System {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_with_options(options: SystemOptions) -> Self {
        let mut memory = Memory::new();
        memory.write_slice(FONT_SPRITES_ADDRESS, font_sprites());

        Self {
            // user programs start at 0x200
            cpu: Cpu {
                pc: RESERVED_SIZE as u16,
                v: Default::default(),
                i: 0,
                stack: Vec::with_capacity(STACK_SIZE),
            },
            delay_timer: Default::default(),
            sound_timer: Default::default(),
            keyboard: Default::default(),
            display: Default::default(),
            memory,
            options,
            stop: Default::default(),
        }
    }

    pub fn controller(&self) -> SystemController {
        SystemController {
            stop_pin: self.stop.clone(),
            kb_controller: self.keyboard.controller(),
        }
    }

    pub fn load_image<P: AsRef<Path>>(&mut self, p: P) -> io::Result<()> {
        let mut r = File::open(p)?;
        let ram = &mut self.memory.as_bytes_mut()[RESERVED_SIZE..];
        let _ = r.read(ram)?;
        Ok(())
    }

    pub fn load_image_bytes(&mut self, bytes: &[u8]) {
        let ram = &mut self.memory.as_bytes_mut()[RESERVED_SIZE..RESERVED_SIZE + bytes.len()];
        ram.copy_from_slice(bytes);
    }

    pub fn start(mut self) -> JoinHandle<()> {
        thread::spawn(move || {
            let _ = self.run();
        })
    }

    pub fn run(&mut self) -> Result<(), SystemError> {
        let mut rng = SmallRng::from_entropy();
        let mut loop_helper =
            LoopHelper::builder().build_with_target_rate(self.options.cpu_frequency_hz);

        while !self.stop.is_raised() {
            let _ = loop_helper.loop_start();
            self.execute_next_inst(&mut rng)?;
            loop_helper.loop_sleep();
        }

        Ok(())
    }

    fn execute_next_inst(&mut self, rng: &mut impl Rng) -> Result<(), SystemError> {
        // health check: PC must be even, otherwise we exit
        /*if self.cpu.pc % 2 != 0 {
            return Err(SystemError::OddPcAddress);
        }*/

        let instr = self
            .memory
            .read_u16(self.cpu.pc)
            .ok_or(SystemError::MemoryReadOverflow)?;
        let opcode = parse_opcode(instr).ok_or(SystemError::UnknownInstruction)?;

        // println!("0x{:04x}: {:04X} {:?}", self.cpu.pc, instr, &opcode);

        match opcode {
            Instr::ClearDisplay => {
                self.display.clear();
            }
            Instr::Return => {
                self.cpu.pc = self.cpu.stack.pop().ok_or(SystemError::StackUnderflow)?;
            }
            Instr::Jump(nnn) => {
                if self.cpu.pc == nnn {
                    return Err(SystemError::SelfJump);
                }
                self.cpu.pc = nnn;
                return Ok(());
            }
            Instr::Call(nnn) => {
                if self.cpu.stack.len() >= 16 {
                    return Err(SystemError::StackOverflow);
                }
                self.cpu.stack.push(self.cpu.pc);
                self.cpu.pc = nnn;
                return Ok(());
            }
            Instr::SkipEqImm(x, kk) => {
                if self.cpu.v[x] == kk {
                    self.cpu.pc += 2;
                }
            }
            Instr::SkipNotEqImm(x, kk) => {
                if self.cpu.v[x] != kk {
                    self.cpu.pc += 2;
                }
            }
            Instr::SkipEqReg(x, y) => {
                if self.cpu.v[x] == self.cpu.v[y] {
                    self.cpu.pc += 2;
                }
            }
            Instr::LoadImm(x, kk) => {
                self.cpu.v[x] = kk;
            }
            Instr::AddImm(x, kk) => {
                // underspecified, what kind of add is it? assume wrapping
                self.cpu.v[x] = self.cpu.v[x].wrapping_add(kk);
            }
            Instr::LoadReg(x, y) => {
                self.cpu.v[x] = self.cpu.v[y];
            }
            Instr::OrReg(x, y) => {
                self.cpu.v[x] |= self.cpu.v[y];
            }
            Instr::AndReg(x, y) => {
                self.cpu.v[x] &= self.cpu.v[y];
            }
            Instr::XorReg(x, y) => {
                self.cpu.v[x] ^= self.cpu.v[y];
            }
            Instr::AddReg(x, y) => {
                let (sum, overflow) = self.cpu.v[x].overflowing_add(self.cpu.v[y]);
                self.cpu.v[x] = sum;
                self.cpu.v[VReg::VF] = overflow as u8;
            }
            Instr::SubReg(x, y) => {
                let (sub, overflow) = self.cpu.v[x].overflowing_sub(self.cpu.v[y]);
                self.cpu.v[x] = sub;
                self.cpu.v[VReg::VF] = !overflow as u8;
            }
            Instr::ShiftRight(x, y) => {
                if self.options.quirks.contains(Quirks::SHIFT_READS_VX) {
                    self.cpu.v[VReg::VF] = self.cpu.v[x] & 1;
                    self.cpu.v[x] >>= 1;
                } else {
                    self.cpu.v[VReg::VF] = self.cpu.v[y] & 1;
                    self.cpu.v[x] = self.cpu.v[y] >> 1;
                }
            }
            Instr::SubN(x, y) => {
                let (sub, overflow) = self.cpu.v[y].overflowing_sub(self.cpu.v[x]);
                self.cpu.v[x] = sub;
                self.cpu.v[VReg::VF] = if overflow { 0 } else { 1 };
            }
            Instr::ShiftLeft(x, y) => {
                if self.options.quirks.contains(Quirks::SHIFT_READS_VX) {
                    self.cpu.v[VReg::VF] = ((self.cpu.v[x] & 0x80) != 0) as u8;
                    self.cpu.v[x] <<= 1;
                } else {
                    self.cpu.v[VReg::VF] = ((self.cpu.v[y] & 0x80) != 0) as u8;
                    self.cpu.v[x] = self.cpu.v[y] << 1;
                }
            }
            Instr::SkipNotEqReg(x, y) => {
                if self.cpu.v[x] != self.cpu.v[y] {
                    self.cpu.pc += 2;
                }
            }
            Instr::LoadI(nnn) => {
                self.cpu.i = nnn;
            }
            Instr::JumpV0(nnn) => {
                self.cpu.pc = nnn.wrapping_add(self.cpu.v[VReg::V0] as u16);
                return Ok(());
            }
            Instr::Random(x, kk) => {
                self.cpu.v[x] = kk & rng.gen::<u8>();
            }
            Instr::Draw(x, y, n) => {
                let bytes = self
                    .memory
                    .read_slice(self.cpu.i, n)
                    .ok_or(SystemError::MemoryReadOverflow)?;

                self.cpu.v[VReg::VF] = if self.options.quirks.contains(Quirks::DRAW_WRAPS_PIXELS) {
                    self.display
                        .draw_sprite_wrapped((self.cpu.v[x], self.cpu.v[y]), bytes)
                } else {
                    self.display
                        .draw_sprite_clipped((self.cpu.v[x], self.cpu.v[y]), bytes)
                } as u8;
            }
            Instr::SkipKeyPressed(x) => {
                if let Some(k) = Key::from(self.cpu.v[x]) {
                    if self.keyboard.is_key_down(k) {
                        self.cpu.pc += 2;
                    }
                }
            }
            Instr::SkipKeyNotPressed(x) => {
                if let Some(k) = Key::from(self.cpu.v[x]) {
                    if !self.keyboard.is_key_down(k) {
                        self.cpu.pc += 2;
                    }
                }
            }
            Instr::LoadDelayTimer(x) => {
                self.cpu.v[x] = self.delay_timer.value();
            }
            Instr::WaitKeyPress(x) => {
                self.cpu.v[x] = self
                    .keyboard
                    .wait_for_key_press()
                    .ok_or(SystemError::Interrupted)? as u8;
            }
            Instr::SetDelayTimer(x) => {
                self.delay_timer.update(self.cpu.v[x]);
            }
            Instr::SetSoundTimer(x) => {
                self.sound_timer.update(self.cpu.v[x]);
            }
            Instr::AddI(x) => {
                // underspecified, what kind of add is it? assume wrapping
                self.cpu.i = self.cpu.i.wrapping_add(self.cpu.v[x] as u16);
            }
            Instr::LoadSprite(x) => {
                self.cpu.i = FONT_SPRITES_ADDRESS + (self.cpu.v[x] as u16 * 5);
            }
            Instr::LoadBCD(x) => {
                let s = format!("{:03}", self.cpu.v[x]);
                let a = s
                    .as_bytes()
                    .iter()
                    .map(|c| c.saturating_sub(b'0'))
                    .collect::<Vec<_>>();
                self.memory.write_slice(self.cpu.i, &a);
            }
            Instr::SaveRegs(x) => {
                self.memory
                    .write_slice(self.cpu.i, &self.cpu.v[0..=x as usize]);
                if !self.options.quirks.contains(Quirks::LOAD_STORE_IGNORES_I) {
                    self.cpu.i += x as u16 + 1;
                }
            }
            Instr::LoadRegs(x) => {
                let s = self
                    .memory
                    .read_slice(self.cpu.i, x as u8 + 1)
                    .ok_or(SystemError::MemoryReadOverflow)?;
                self.cpu.v[0..=x as usize].copy_from_slice(s);
                if !self.options.quirks.contains(Quirks::LOAD_STORE_IGNORES_I) {
                    self.cpu.i += x as u16 + 1;
                }
            }
        }

        self.cpu.pc += 2;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn stop_works() {
        let mut chip8 = System::new();
        let ctrl = chip8.controller();

        let image = [0x00, 0xE0, 0x12, 0x00];
        chip8.load_image_bytes(&image);

        let j = chip8.start();

        sleep(Duration::from_millis(200));
        ctrl.stop();

        let r = j.join();

        assert!(r.is_ok());
    }

    #[test]
    fn stop_when_waiting_for_key_press_works() {
        let mut chip8 = System::new();
        let ctrl = chip8.controller();

        let image = [0xF1, 0x0A];
        chip8.load_image_bytes(&image);

        let j = chip8.start();

        sleep(Duration::from_millis(200));
        ctrl.stop();

        let r = j.join();

        assert!(r.is_ok());
    }
}
