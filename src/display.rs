use minifb::{Key, Window, WindowOptions};
use std::time::Duration;

const CHIP8_WIDTH: usize = 64;
const CHIP8_HEIGHT: usize = 32;
// Our display is 10x bigger than CHIP-8 in every direction
const SCALE: usize = 10;
pub const ON: u32 = 0xFF_FF_FF; // white
pub const OFF: u32 = 0; // black
const SIXTY_FPS: Duration = Duration::from_micros(16600);

/// A framebuffer that pretends to be 10x smaller than it is. This lets it
/// display a 64x32 screen at 640x320. It scales pixels proportionately, too:
/// flipping a logical pixel at (0, 0) flips all 100 physical pixels from (0, 0)
/// to (9, 9).
#[derive(Debug, Clone, PartialEq)]
pub struct ScaledFramebuffer {
    buffer: Vec<u32>,
    pub true_width: usize,
    pub true_height: usize,
}

impl ScaledFramebuffer {
    /// Initialize with the CHIP-8's width and height.
    pub fn new() -> Self {
        Self::with_size(CHIP8_WIDTH, CHIP8_HEIGHT)
    }

    /// Create a framebuffer from logical pixels. So for the CHIP-8, which has a
    /// 64x32 screen, pass in 64 and 32, and it will draw it on a 640x320
    /// display.
    fn with_size(logical_width: usize, logical_height: usize) -> Self {
        let scaled_width = logical_width * SCALE;
        let scaled_height = logical_height * SCALE;
        Self {
            // Start with a blank screen
            buffer: vec![OFF; scaled_width * scaled_height],
            true_width: scaled_width,
            true_height: scaled_height,
        }
    }

    pub fn as_bytes(&self) -> &Vec<u32> {
        &self.buffer
    }

    /// Get the value of a pixel at logical location (x, y).
    /// It only checks one physical pixel, and assumes all of the other pixels
    /// that make up this one logical pixel have the same value.
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        self.buffer[(SCALE * x) + (SCALE * y * self.true_width)]
    }

    /// Set the value of a pixel at logical location (x, y).
    /// Behind the scenes, this actually sets `SCALE * SCALE` physical pixels because
    /// it sets `SCALE` pixels across times `SCALE` pixels down.
    pub fn set_pixel(&mut self, x: usize, y: usize, new_value: u32) {
        for x_offset in 0..SCALE {
            let scaled_x = SCALE * x + x_offset;
            for y_offset in 0..SCALE {
                let scaled_y = (SCALE * y + y_offset) * self.true_width;
                self.buffer[scaled_x + scaled_y] = new_value;
            }
        }
    }

    /// XOR a given pixel at logical location (x, y) with the incoming input bit
    /// (true = 1, false = 0).
    /// If the input bit is 0, does nothing.
    /// If the input bit is 1, flips the value at (x, y).
    /// Returns true if a set pixel was changed to unset, and false otherwise.
    pub fn xor(&mut self, input_bit: bool, x: usize, y: usize) -> bool {
        if !input_bit {
            debug!("xor ({}, {}): input is 0, not doing anything", x, y);
            return false;
        }

        if self.get_pixel(x, y) == ON {
            debug!("xor ({}, {}): Flipping from ON to OFF", x, y);
            self.set_pixel(x, y, OFF);
            true
        } else {
            debug!("xor ({}, {}): Flipping from OFF to ON", x, y);
            self.set_pixel(x, y, ON);
            false
        }
    }

    /// Pretty-print a grid of 1 (on) and 0 (off) that represents the screen.
    /// Prints physical pixels, for debugging.
    pub fn pretty_print_physical(&self) -> String {
        let mut result = vec![];
        for (index, row) in self.buffer.chunks_exact(self.true_width).enumerate() {
            let column = row
                .iter()
                .map(|b| format!("{}", if b == &ON { 1 } else { 0 }))
                .collect::<Vec<_>>();
            result.push(format!("{} {}", index, column.join("")));
        }
        result.join("\n")
    }

    /// Draw the given sprite at logical location (x, y).
    /// The sprite is interpreted as a bit pattern with 0 = off and 1 = on.
    /// For example, these 3 bytes would draw a "0":
    /// 00111100
    /// 00100100
    /// 00111100
    /// Returns true if a set pixel was changed to unset, and false otherwise.
    pub fn draw_sprite_at(&mut self, x: usize, y: usize, sprite: &[u8]) -> bool {
        let mut changed_from_on_to_off = false;
        let bit_is_set = |byte: &u8, position: u8| ((byte & (1 << position)) >> position) == 1;
        for (y_offset, row) in sprite.iter().enumerate() {
            // Move left across the bits of the byte:
            // 11010001
            // ^-------
            // 11010001
            //  ^------
            for x_offset in 0..=7 {
                let input_bit = bit_is_set(row, (7 - x_offset) as u8);
                let result = self.xor(input_bit, x + x_offset, y + y_offset);
                changed_from_on_to_off = result || changed_from_on_to_off;
            }
        }
        changed_from_on_to_off
    }
}

/// It knows how to draw a `ScaledFramebuffer` to the screen.
pub struct Display {
    window: Window,
}

impl Display {
    pub fn new(width: usize, height: usize) -> Self {
        let mut window = Window::new(
            "CHIP-8 - ESC to exit",
            width,
            height,
            WindowOptions::default(),
        )
        .unwrap_or_else(|e| panic!("{}", e));
        window.limit_update_rate(Some(SIXTY_FPS));

        Self { window }
    }

    /// Usage: `while display.is_running { ... }
    pub fn is_running(&self) -> bool {
        self.window.is_open() && !self.window.is_key_down(Key::Escape)
    }

    /// Update the screen with the new buffer data.
    pub fn draw(&mut self, buffer: &ScaledFramebuffer) {
        self.window
            .update_with_buffer(buffer.as_bytes(), buffer.true_width, buffer.true_height)
            .unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Assert on all 100 pixels (10 in x direction, 10 in y direction) that a single logical pixel corresponds to.
    fn assert_pixel(fb: &ScaledFramebuffer, x: usize, y: usize, color: u32) {
        for x_offset in 0..SCALE {
            for y_offset in 0..SCALE {
                let scaled_y = (SCALE * y + y_offset) * fb.true_width;
                let scaled_x = (SCALE * x) + x_offset;
                assert_eq!(fb.buffer[scaled_y + scaled_x], color);
            }
        }
    }

    #[test]
    fn turn_pixel_on() {
        let mut fb = ScaledFramebuffer::with_size(5, 5);
        let x = 2;
        let y = 2;
        let flipped_to_off = fb.xor(true, x, y);

        assert_eq!(flipped_to_off, false);
        assert_pixel(&fb, x, y, ON);
    }

    #[test]
    fn turn_pixel_off() {
        let mut fb = ScaledFramebuffer::with_size(5, 5);
        let x = 2;
        let y = 2;
        fb.xor(true, x, y);
        fb.xor(true, x, y);

        assert_pixel(&fb, x, y, OFF);
    }

    #[test]
    fn xor_detect_when_pixel_flips_from_on_to_off() {
        let mut fb = ScaledFramebuffer::with_size(5, 5);
        let x = 2;
        let y = 2;

        assert_eq!(fb.xor(true, x, y), false);
        assert_eq!(fb.xor(true, x, y), true);
    }

    #[test]
    fn draw_sprite() {
        #[rustfmt::skip]
        let sprite = &[
            0b11110000,
            0b10010000,
            0b10010000,
            0b10010000,
            0b11110000,
        ];
        let mut fb = ScaledFramebuffer::with_size(8, 5);
        fb.draw_sprite_at(0, 0, sprite);

        // First row
        for x in 0..4 {
            assert_pixel(&fb, x, 0, ON);
        }
        // Rows 2-4
        for y in 1..3 {
            assert_pixel(&fb, 0, y, ON);
            assert_pixel(&fb, 3, y, ON);
        }
        // Last row
        for x in 0..4 {
            assert_pixel(&fb, x, 4, ON);
        }

        // Check that some of the pixels that shouldn't be on, are not on
        assert_pixel(&fb, 4, 0, OFF);
        assert_pixel(&fb, 1, 1, OFF);
    }

    #[test]
    fn draw_two_sprites() {
        #[rustfmt::skip]
        let first_sprite = &[
            0b11110000,
            0b01000000,
            0b11110000,
            0b00000000,
            0b00000000,
        ];
        #[rustfmt::skip]
        let second_sprite = &[
            0b11110000,
            0b10000000,
            0b11100000,
            0b10000000,
            0b11110000,
        ];
        let mut fb = ScaledFramebuffer::with_size(8, 5);
        fb.draw_sprite_at(0, 0, first_sprite);
        fb.draw_sprite_at(0, 0, second_sprite);

        let expected = vec![
            vec![OFF; 8],
            vec![ON, ON, OFF, OFF, OFF, OFF, OFF, OFF],
            vec![OFF, OFF, OFF, ON, OFF, OFF, OFF, OFF],
            vec![ON, OFF, OFF, OFF, OFF, OFF, OFF, OFF],
            vec![ON, ON, ON, ON, OFF, OFF, OFF, OFF],
        ];

        for (y, row) in expected.iter().enumerate() {
            for (x, value) in row.iter().enumerate() {
                assert_pixel(&fb, x, y, *value);
            }
        }
    }

    #[test]
    fn draw_sprite_detect_when_pixel_flips_from_on_to_off() {
        let sprite1 = &[0b11110000];
        let sprite2 = &[0b00010000];
        let mut fb = ScaledFramebuffer::with_size(8, 1);

        assert_eq!(fb.draw_sprite_at(0, 0, sprite1), false);
        assert_eq!(fb.draw_sprite_at(0, 0, sprite2), true);
    }
}
