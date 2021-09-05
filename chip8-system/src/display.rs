use crate::port::OutputPort;
use bitvec::prelude::*;
use crossbeam_channel::{Receiver, Sender};

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
pub const DISPLAY_BUFFER_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;

pub type PixelBuffer = BitVec;

pub fn pixel_buffer() -> PixelBuffer {
    bitvec![0; DISPLAY_BUFFER_SIZE]
}

pub enum DisplayMessage {
    Clear,
    Update(PixelBuffer),
}

pub(crate) struct DisplayBuffer {
    pixels: PixelBuffer,
    sender: Sender<DisplayMessage>,
    receiver: Receiver<DisplayMessage>,
}

impl Default for DisplayBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayBuffer {
    pub fn new() -> Self {
        let (s, r) = crossbeam_channel::unbounded();

        Self {
            pixels: pixel_buffer(),
            sender: s,
            receiver: r,
        }
    }

    pub fn clear(&mut self) {
        self.pixels = pixel_buffer();
        let _ = self.sender.try_send(DisplayMessage::Clear);
    }

    pub fn draw_sprite(&mut self, (x, y): (u8, u8), sprite: &[u8]) -> bool {
        let mut collision = false;
        for (row, &data) in sprite.iter().enumerate() {
            let py = (y as usize + row) % DISPLAY_HEIGHT;
            for bit in 0..8u8 {
                let px = (x as usize + bit as usize) % DISPLAY_WIDTH;
                let i = DISPLAY_WIDTH * py + px;

                if let Some(mut pixel) = self.pixels.get_mut(i) {
                    let prev = *pixel;
                    let sprite = bit_at(data, 7u8 - bit);
                    let new = prev ^ sprite;
                    *pixel = new;

                    // collision flag set to true if at least a pixel has been switched
                    // from 1 to 0 during the draw operation
                    if prev && !new {
                        collision = true;
                    }
                }
            }
        }
        let _ = self
            .sender
            .try_send(DisplayMessage::Update(self.pixels.clone()));

        collision
    }
}

impl OutputPort<DisplayMessage> for DisplayBuffer {
    fn output(&self) -> Receiver<DisplayMessage> {
        self.receiver.clone()
    }
}

fn bit_at(input: u8, n: u8) -> bool {
    if n < 8 {
        input & (1 << n) != 0
    } else {
        false
    }
}

#[allow(unused)]
fn debug_pixels() -> PixelBuffer {
    let mut pixels = bitvec![];

    for _ in 0..DISPLAY_HEIGHT / 2 {
        let mut lines = bitvec![
            0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0,
            1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1,
            0, 1, 0, 1, 0, 1, // line
            1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1,
            0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0,
            1, 0, 1, 0, 1, 0
        ];

        pixels.append(&mut lines);
    }

    pixels
}

pub const FONT_SPRITES_ADDRESS: u16 = 0;

#[rustfmt::skip]
pub fn font_sprites() -> &'static [u8] {
    &[
        /* 0 */
        0b11110000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b11110000,
        /* 1 */
        0b00100000,
        0b01100000,
        0b00100000,
        0b00100000,
        0b01110000,
        /* 2 */
        0b11110000,
        0b00010000,
        0b11110000,
        0b10000000,
        0b11110000,
        /* 3 */
        0b11110000,
        0b00010000,
        0b11110000,
        0b00010000,
        0b11110000,
        /* 4 */
        0b10010000,
        0b10010000,
        0b11110000,
        0b00010000,
        0b00010000,
        /* 5 */
        0b11110000,
        0b10000000,
        0b11110000,
        0b00010000,
        0b11110000,
        /* 6 */
        0b11110000,
        0b10000000,
        0b11110000,
        0b10010000,
        0b11110000,
        /* 7 */
        0b11110000,
        0b00010000,
        0b00100000,
        0b01000000,
        0b01000000,
        /* 8 */
        0b11110000,
        0b10010000,
        0b11110000,
        0b10010000,
        0b11110000,
        /* 9 */
        0b11110000,
        0b10010000,
        0b11110000,
        0b00010000,
        0b11110000,
        /* A */
        0b11110000,
        0b10010000,
        0b11110000,
        0b10010000,
        0b10010000,
        /* B */
        0b11100000,
        0b10010000,
        0b11100000,
        0b10010000,
        0b11100000,
        /* C */
        0b11110000,
        0b10000000,
        0b10000000,
        0b10000000,
        0b11110000,
        /* D */
        0b11100000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b11100000,
        /* E */
        0b11110000,
        0b10000000,
        0b11110000,
        0b10000000,
        0b11110000,
        /* F */
        0b11110000,
        0b10000000,
        0b11110000,
        0b10000000,
        0b10000000,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_at_works() {
        let a = 0b1001;
        assert!(bit_at(a, 0));
        assert!(!bit_at(a, 1));
        assert!(!bit_at(a, 2));
        assert!(bit_at(a, 3));
    }
}
