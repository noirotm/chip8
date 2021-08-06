use std::convert::TryInto;

pub const MEMORY_SIZE: usize = 4096;
pub const RESERVED_SIZE: usize = 512;

pub(crate) struct Memory {
    bytes: [u8; MEMORY_SIZE],
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    pub fn new() -> Self {
        Self {
            bytes: [0; MEMORY_SIZE],
        }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    pub fn read_u16(&self, addr: u16) -> Option<u16> {
        let addr = addr as usize;
        let b = self.bytes.get(addr..=addr + 1)?.try_into().ok()?;
        Some(u16::from_be_bytes(b))
    }

    pub fn read_slice(&self, addr: u16, n: u8) -> Option<&[u8]> {
        let addr = addr as usize;
        self.bytes.get(addr..addr + (n as usize))
    }

    pub fn write_slice(&mut self, addr: u16, data: &[u8]) {
        let addr = addr as usize;
        self.bytes[addr..addr + data.len()].copy_from_slice(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u16_big_endian_works() {
        let mut m = Memory::new();
        m.write_slice(0x200, &[0xAB]);
        m.write_slice(0x201, &[0xCD]);
        let v = m.read_u16(0x200);
        assert_eq!(v, Some(0xABCD));
    }
}
