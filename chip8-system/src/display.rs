use crate::port::{Shared, SharedData};
use bitvec::prelude::*;
use std::sync::{Arc, RwLock};

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
pub const DISPLAY_BUFFER_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;

pub type PixelBuffer = BitVec;

pub(crate) struct DisplayBuffer {
    pixels: Shared<PixelBuffer>,
}

impl Default for DisplayBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayBuffer {
    pub fn new() -> Self {
        Self {
            pixels: Arc::new(RwLock::new(bitvec![0; DISPLAY_BUFFER_SIZE])),
        }
    }

    pub fn clear(&mut self) {
        if let Ok(mut pixels) = self.pixels.write() {
            *pixels = bitvec![0; DISPLAY_BUFFER_SIZE];
        }
    }

    pub fn draw_sprite(&mut self, (x, y): (u8, u8), sprite: &[u8]) -> bool {
        let mut collision = false;
        if let Ok(mut pixels) = self.pixels.write() {
            for (row, &data) in sprite.iter().enumerate() {
                let py = (y as usize + row) % DISPLAY_HEIGHT;
                for bit in 0..8u8 {
                    let px = (x as usize + bit as usize) % DISPLAY_WIDTH;
                    let i = DISPLAY_WIDTH * py + px;

                    if let Some(mut pixel) = pixels.get_mut(i) {
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
        }

        collision
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

impl SharedData<PixelBuffer> for DisplayBuffer {
    fn data(&self) -> Shared<PixelBuffer> {
        self.pixels.clone()
    }
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
