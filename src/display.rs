use log::Level::Debug;
use minifb::{Key, Window, WindowOptions};
use std::time::Duration;

const CHIP8_WIDTH: usize = 64;
const CHIP8_HEIGHT: usize = 32;
// Our display is 10x bigger than CHIP-8 in every direction
const SCALE: usize = 10;
const ON: u32 = 0xFF_FF_FF; // white
const OFF: u32 = 0; // black
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

    /// Flip a given pixel. If it's on, turn it off. If it's off, turn it on.
    pub fn flip(&mut self, x: usize, y: usize) {
        debug!("Flipping pixel at logical location ({}, {})", x, y);

        // Pixels are scaled by SCALE amount (e.g. 64x32 -> 640x320).
        // So we set SCALE pixels across and SCALE pixels down for each "pixel".
        for x_offset in 0..SCALE {
            let scaled_x = SCALE * x + x_offset;
            for y_offset in 0..SCALE {
                let scaled_y = (SCALE * y + y_offset) * self.true_width;
                let old_value = self.buffer[scaled_x + scaled_y];
                if old_value == ON {
                    self.buffer[scaled_x + scaled_y] = OFF;
                } else {
                    self.buffer[scaled_x + scaled_y] = ON;
                }
            }
        }
    }

    /// Pretty-print a grid of 1 (on) and 0 (off) that represents the screen.
    fn pretty_print(&self) -> String {
        let mut result = vec![];
        for row in self.buffer.chunks_exact(self.true_width) {
            let column = row
                .iter()
                .map(|b| format!("{}", if b == &ON { 1 } else { 0 }))
                .collect::<Vec<_>>();
            result.push(column.join(" "));
        }
        result.join("\n")
    }

    /// Draw the given sprite at (x, y).
    /// The sprite is interpreted as a bit pattern with 0 = off and 1 = on.
    /// For example, these 3 bytes would draw a "0":
    /// 00111100
    /// 00100100
    /// 00111100
    pub fn draw_sprite_at(&mut self, x: usize, y: usize, sprite: &[u8]) {
        if log_enabled!(Debug) {
            let pretty_sprite = sprite
                .iter()
                .map(|b| format!("{:08b}", b))
                .collect::<Vec<_>>()
                .join("\n");
            debug!("Sprite: {}", pretty_sprite);
        }
        let bit_is_set = |byte: &u8, position: u8| ((byte & (1 << position)) >> position) == 1;
        for (y_offset, row) in sprite.iter().enumerate() {
            // Move left across the bits of the byte:
            // 11010001
            // ^-------
            // 11010001
            //  ^------
            for x_offset in 0..=7 {
                if bit_is_set(row, (7 - x_offset) as u8) {
                    self.flip(x + x_offset, y + y_offset);
                }
            }
        }
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

    /// Draw the buffer to the screen.
    pub fn draw(&mut self, buffer: &ScaledFramebuffer) {
        self.window
            .update_with_buffer(buffer.as_bytes(), buffer.true_width, buffer.true_height)
            .unwrap();
    }
}

mod test {
    #[allow(unused_imports)]
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
        fb.flip(x, y);

        assert_pixel(&fb, x, y, ON);
    }

    #[test]
    fn turn_pixel_off() {
        let mut fb = ScaledFramebuffer::with_size(5, 5);
        let x = 2;
        let y = 2;
        fb.flip(x, y);
        fb.flip(x, y);

        assert_pixel(&fb, x, y, OFF);
    }

    #[test]
    fn draw_sprite() {
        #[cfg_attr(rustfmt, rustfmt_skip)]
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
}
