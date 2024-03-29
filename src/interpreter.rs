use crate::{
    display::{Display, ScaledFramebuffer},
    instruction::{Instruction, Instruction::*},
};
use crate::{error::Chip8Error, instruction::Register};
use log::Level::Debug;
use rand::{Rng, RngCore};
use std::convert::TryFrom;

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// 4KB = 4096 bytes of RAM.
    /// The first 512 bytes (0x000 to 0x1FF) are for the interpreter and not to be used.
    /// Most CHIP-8 programs start at 0x200 = 512.
    /// So, the main memory is from 0x200 to 0xFFF.
    memory: Vec<u8>,
    /// Chip-8 has 16 general purpose 8-bit registers, usually referred to as Vx, where x is a hexadecimal digit (0 through F).
    registers: Vec<u8>,
    /// A 16-bit register called I. This register is generally used to
    /// store memory addresses, so only the lowest (rightmost) 12 bits
    /// are usually used.
    i: u16,
    /// Program counter
    pc: u16,
    /// Stack pointer
    sp: u8,
    /// The stack is an array of 16 16-bit values, used to store the address that
    /// the interpreter should return to when finished with a subroutine.
    /// Chip-8 allows for up to 16 levels of nested subroutines.
    stack: Vec<u16>,

    /// The framebuffer
    buffer: ScaledFramebuffer,
}

impl State {
    /// Create a new State with the given program.
    pub fn with_program(program: &[u8]) -> Self {
        // Program space is from 0x200 to 0xFFF.
        assert!(program.len() <= (0xFFF - 0x200));

        // Start with 0x200 empty bytes, then add the program at the end
        let interpreter_area = &[0; 0x200];
        let memory = [interpreter_area, program].concat();

        Self {
            memory,
            registers: vec![0; 16],
            i: 0,
            pc: 0x200,
            sp: 0,
            stack: vec![0; 16],
            buffer: ScaledFramebuffer::new(),
        }
    }

    /// Set the given register to the given value.
    fn set_register<U: Into<Register>>(&mut self, unconverted: U, value: u8) {
        let register = unconverted.into();
        self.registers[register.0 as usize] = value;
    }

    /// Get the value in the given register.
    fn get_register<U: Into<Register>>(&self, unconverted: U) -> u8 {
        let register = unconverted.into();
        self.registers[register.0 as usize]
    }

    /// Increase I by the value in the given register.
    fn increase_i(&mut self, register: &Register) {
        self.i += self.get_register(*register) as u16;
    }

    /// Set the program counter to the given address.
    fn set_pc(&mut self, address: u16) {
        self.pc = address;
    }

    /// Increment the stack pointer and push a value onto the top of the stack.
    fn push_onto_stack(&mut self, value: u16) {
        self.stack[self.sp as usize] = value;
        self.sp += 1;
    }

    /// Decrement the stack pointer and return the value that it used to point to.
    fn pop_off_stack(&mut self) -> u16 {
        if self.sp == 0 {
            panic!("Cannot decrement stack pointer, already at 0");
        }
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    fn next_chunk(&self) -> Option<u16> {
        let one = self.memory.get(self.pc as usize)?;
        let two = self.memory.get((self.pc + 1) as usize)?;
        Some(u16::from_be_bytes([*one, *two]))
    }
}

/// Run the entire program, forever.
pub fn run(state: &mut State, verbosely: bool) -> Result<&mut State, Chip8Error> {
    let mut display = Display::new(state.buffer.true_width, state.buffer.true_height);
    let rng = rand::thread_rng();

    while display.is_running() {
        match state.next_chunk() {
            Some(chunk) => {
                // Advance by 2 bytes since 1 chunk is 2 bytes
                state.pc += 2;
                let instruction = Instruction::try_from(chunk)?;
                execute(state, &instruction, Box::new(rng), verbosely)?;
                display.draw(&state.buffer);
                trace!("{}", state.buffer.pretty_print_physical());
            }
            None => break,
        }
    }
    Ok(state)
}

// Do one thing in the interpreter (run one instruction) and return the changed state.
// Useful for testing.
#[cfg(test)]
fn tick(state: &mut State, rng: impl RngCore) -> Result<&mut State, Chip8Error> {
    let chunk = state.next_chunk().unwrap();
    // Advance by 2 bytes since 1 chunk is 2 bytes
    state.pc += 2;
    let instruction = Instruction::try_from(chunk)?;
    execute(state, &instruction, rng, false)?;
    Ok(state)
}

/// Execute a single instruction and return the changed `State`.
fn execute<'a>(
    state: &'a mut State,
    instruction: &Instruction,
    mut rng: impl RngCore,
    verbosely: bool,
) -> Result<&'a mut State, Chip8Error> {
    if verbosely {
        // Subtract 2 to get the value for this instruction, because we add 2 before running `execute`
        println!("[{:03X}], {}", state.pc - 2, instruction);
    }
    match instruction {
        SYS() => {
            if verbosely {
                println!("\tIgnoring");
            }
        }
        RET() => {
            let old_pc = state.pc;
            state.pc = state.pop_off_stack();
            if verbosely {
                println!("\tChanged pc from {:04X} -> {:04X}", old_pc, state.pc);
            }
        }
        JP(address) => {
            let old_pc = state.pc;
            state.set_pc((*address).into());
            if verbosely {
                println!("\tChanged pc from {:04X} -> {:04X}", old_pc, state.pc);
            }
        }
        CALL(address) => {
            let old_pc = state.pc;
            state.push_onto_stack(state.pc);
            if verbosely {
                println!("\tPushed pc ({:04X}) onto stack", state.pc);
            }
            state.set_pc((*address).into());
            if verbosely {
                println!("\tChanged pc from {:04X} -> {:04X}", old_pc, state.pc);
            }
        }
        SEByte(register, byte) => {
            let register_value = state.get_register(*register);
            if register_value == *byte {
                state.pc += 2;
                if verbosely {
                    println!("\tSkipping ahead, V{:X} == {:02X}", register.0, byte);
                }
            } else if verbosely {
                println!(
                    "\tNot skipping, V{:X} is {:02X} (would skip if it were {:02X})",
                    register.0, register_value, byte
                );
            }
        }
        SNEByte(register, byte) => {
            let register_value = state.get_register(*register);
            if register_value != *byte {
                state.pc += 2;
                if verbosely {
                    println!("\tSkipping ahead, V{:X} != {:02X}", register.0, byte);
                }
            } else if verbosely {
                println!(
                    "\tNot skipping, V{:X} is {:02X} (would skip if it were not {:02X})",
                    register.0, register_value, byte
                );
            }
        }
        SERegister(register_x, register_y) => {
            let register_x_value = state.get_register(*register_x);
            let register_y_value = state.get_register(*register_y);
            if register_x_value == register_y_value {
                state.pc += 2;
                if verbosely {
                    println!(
                        "\tSkipping ahead, V{:X} == V{:X}",
                        register_x.0, register_y.0
                    );
                }
            } else if verbosely {
                println!(
                    "\tNot skipping, V{:X} is {:02X} (would skip if it were {:02X})",
                    register_x.0, register_x_value, register_y_value
                );
            }
        }
        SNERegister(register_x, register_y) => {
            let register_x_value = state.get_register(*register_x);
            let register_y_value = state.get_register(*register_y);
            if register_x_value != register_y_value {
                state.pc += 2;
                if verbosely {
                    println!(
                        "\tSkipping ahead, V{:X} != V{:X}",
                        register_x.0, register_y.0
                    );
                }
            } else if verbosely {
                println!(
                    "\tNot skipping, V{:X} is {:02X} (would skip if it were any other value)",
                    register_x.0, register_x_value
                );
            }
        }
        LDByte(register, value) => {
            state.set_register(*register, *value);
            if verbosely {
                println!("\tSet register V{:X} to {:02X}", register.0, value);
            }
        }
        ADDByte(register, addend) => {
            let old_value = state.get_register(*register);
            let new_value = addend.wrapping_add(old_value);
            state.set_register(*register, new_value);
            if verbosely {
                println!(
                    "\tChanged register V{:X} from {:02X} -> {:02X}",
                    register.0, old_value, new_value
                );
            }
        }
        ADDRegister(register_x, register_y) => {
            let value_x = state.get_register(*register_x);
            let value_y = state.get_register(*register_y);
            let (result, did_overflow) = value_x.overflowing_add(value_y);
            if did_overflow {
                state.set_register(0xF, 1);
            }
            state.set_register(*register_x, result);
            if verbosely {
                println!(
                    "\tChanged register V{:X} from {:02X} -> {:02X} (VF = {})",
                    register_x.0,
                    value_x,
                    result,
                    if did_overflow { 1 } else { 0 }
                );
            }
        }
        LDI(address) => {
            let value = (*address).into();
            state.i = value;
            if verbosely {
                println!("\tSet register I to {:04X}", value);
            }
        }
        RND(register, byte) => {
            let random_value: u8 = rng.gen();
            let new_value = random_value & byte;
            state.set_register(*register, new_value);
            if verbosely {
                println!(
                    "\tSet register V{:X} to {:X} (= {:X} & {:X})",
                    register.0, new_value, random_value, byte
                );
            }
        }
        DRW(register_x, register_y, n) => {
            let x = state.get_register(*register_x);
            let y = state.get_register(*register_y);
            let slice_start = state.i as usize;
            let slice_end = slice_start + (*n as usize);
            let sprite = &state.memory[slice_start..slice_end];
            let flipped_from_off_to_on =
                state.buffer.draw_sprite_at(x as usize, y as usize, sprite);
            if verbosely || log_enabled!(Debug) {
                let pretty_sprite = sprite
                    .iter()
                    .map(|byte| format!("\t{:08b}", byte))
                    .collect::<Vec<_>>()
                    .join("\n");
                if verbosely {
                    println!(
                        "\tDrawing at ({}, {}) with sprite data (VF set to {}):\n{}",
                        x,
                        y,
                        if flipped_from_off_to_on { 1 } else { 0 },
                        pretty_sprite,
                    );
                } else if log_enabled!(Debug) {
                    debug!(
                        "\tDrawing at ({}, {}) with sprite data (VF set to {}):\n{}",
                        x,
                        y,
                        if flipped_from_off_to_on { 1 } else { 0 },
                        pretty_sprite,
                    );
                }
            }
            if flipped_from_off_to_on {
                state.set_register(0xF, 1);
            } else {
                state.set_register(0xF, 0);
            }
        }
        ADDI(register) => {
            let old_value = state.i;
            state.increase_i(register);
            let new_value = state.i;
            if verbosely {
                println!("\tChanged I from {:02X} -> {:02X}", old_value, new_value);
            }
        }
        UNKNOWN(bytes) => {
            panic!("Unknown instruction: {:04X}", bytes);
        }
    }
    Ok(state)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::display;

    // Build a program by inserting u16s (as u8s) at the given address and
    // address+1, with everything else filled with zeroes.
    // Note that the program space starts at 0x200, so instructions will be
    // inserted at 0x200 + your address. This is especially important when
    // testing JP/RET.
    // For example: `build_program(vec![(0x300, 0x00EE)])` creates a program
    // with a RET instruction filling the bytes at 0x500 and 0x501.
    fn build_program(addresses_and_chunks: &[(usize, u16)]) -> Vec<u8> {
        let mut program = vec![0; 0xFFF - 0x200];
        addresses_and_chunks
            .iter()
            .copied()
            .for_each(|(address, instruction)| {
                let [b1, b2] = u16::to_be_bytes(instruction);
                program[address] = b1;
                program[address + 1] = b2;
            });
        program
    }

    fn run(chunks: &[u16]) -> State {
        let mut state = build_state_with_program(chunks);
        for _ in chunks {
            tick(&mut state, testing_rng()).unwrap();
        }
        state
    }

    fn build_state_with_program(chunks: &[u16]) -> State {
        // Every instruction is 2 bytes, so generate 0, 2, 4, etc
        let addresses = (0..).filter(|x| x % 2 == 0).take(chunks.len());
        let result = addresses.zip(chunks.iter().copied()).collect::<Vec<_>>();

        State::with_program(&build_program(result.as_slice()))
    }

    fn build_state_with_program_with_custom_offsets(
        addresses_and_chunks: &[(usize, u16)],
    ) -> State {
        State::with_program(&build_program(addresses_and_chunks))
    }

    // A random-number generator with a pre-determined seed.
    fn testing_rng() -> impl RngCore {
        use rand::SeedableRng;
        rand::rngs::StdRng::seed_from_u64(0)
    }

    #[test]
    fn sys_ignored_advances_pc() {
        let state = run(&[SYS().into()]);
        assert_eq!(state.pc, 0x202);
    }

    fn r(n: u8) -> Register {
        Register(n)
    }

    #[test]
    fn call_subroutine_and_return() {
        let program = &[
            // CALL: Increment SP, put current PC (0x200 + 2 = 0x202) on top of stack, set PC to 0x300
            (0, CALL(0x300.into()).into()),
            // At 0x100 (+ 0x200 = 0x300 in the total program memory), do LD 1, 20
            (0x100, LDByte(r(0x1), 0x20).into()),
            // Now RET(urn): Set PC to top of stack (0x202) substract 1 from SP
            (0x102, RET().into()),
        ];
        let mut state = build_state_with_program_with_custom_offsets(program);

        for _ in 0..program.len() {
            tick(&mut state, testing_rng()).unwrap();
        }

        assert_eq!(state.pc, 0x202);
        assert_eq!(state.sp, 0);
        assert_eq!(state.get_register(0x1), 0x20);
    }

    #[test]
    fn jp_addr() {
        let state = run(&[JP(0xBCD.into()).into()]);
        assert_eq!(state.pc, 0xBCD);
    }

    #[test]
    fn ld_vx() {
        let state = run(&[LDByte(r(0xD), 0x12).into()]);
        assert_eq!(state.get_register(0xD), 0x12);
    }

    #[test]
    fn ld_i() {
        let state = run(&[LDI(0x400.into()).into()]);
        assert_eq!(state.i, 0x400);
    }

    #[test]
    fn add_byte() {
        #[rustfmt::skip]
        let state = run(&[
            LDByte(r(0xD), 0x12).into(),
            ADDByte(r(0xD), 0x12).into()
        ]);
        assert_eq!(state.get_register(0xD), 0x24);
    }

    #[test]
    fn add_byte_with_overflow() {
        #[rustfmt::skip]
        let state = run(&[
            LDByte(r(0xD), 0x12).into(),
            ADDByte(r(0xD), 0xFF).into()
        ]);
        // Expect it to wrap around
        assert_eq!(state.get_register(0xD), 0x11);
    }

    #[test]
    fn sne_byte() {
        let state = run(&[
            LDByte(r(0xD), 0x12).into(),
            SNEByte(r(0xD), 0x00).into(),
            // This should be skipped
            LDByte(r(0x1), 0x00).into(),
            // This one should run
            LDByte(r(0x1), 0xFF).into(),
        ]);
        assert_eq!(state.get_register(0x1), 0xFF);
    }

    #[test]
    fn se_byte() {
        let state = run(&[
            LDByte(r(0xD), 0x12).into(),
            SEByte(r(0xD), 0x12).into(),
            // This should be skipped
            LDByte(r(0x1), 0x00).into(),
            // This one should run
            LDByte(r(0x1), 0xFF).into(),
        ]);
        assert_eq!(state.get_register(0x1), 0xFF);
    }

    #[test]
    fn se_register() {
        let state = run(&[
            LDByte(r(0xA), 0x12).into(),
            LDByte(r(0xB), 0x12).into(),
            SERegister(r(0xA), r(0xB)).into(),
            // This should be skipped
            LDByte(r(0x1), 0x00).into(),
            // This one should run
            LDByte(r(0x1), 0xFF).into(),
        ]);
        assert_eq!(state.get_register(0x1), 0xFF);
    }

    #[test]
    fn sne_register() {
        let state = run(&[
            LDByte(r(0xA), 0x12).into(),
            LDByte(r(0xB), 0x12).into(),
            SNERegister(r(0xA), r(0xC)).into(),
            // This should be skipped
            LDByte(r(0x1), 0x12).into(),
            // This should run
            LDByte(r(0x1), 0xFF).into(),
        ]);
        assert_eq!(state.get_register(0x1), 0xFF);
    }

    #[test]
    fn rnd() {
        #[rustfmt::skip]
        let state = run(&[
            LDByte(r(0x1), 0x00).into(),
            RND(r(0x1), 0xFF).into(),
        ]);
        // The testing RNG will always generate 0xB2 as its first u8. 0xB2 &
        // 0xFF == 0xB2
        assert_eq!(state.get_register(0x1), 0xB2);
    }

    #[test]
    fn drw_with_vf_flip_to_1() {
        let sprite1: u8 = 0b11110000;
        // Sprite 2 intentionally has exactly 1 bit that's the same as sprite 1,
        // so that when we draw sprite1 then sprite2, it draws a pixel as set,
        // then unset, causing VF to be set to 1.
        let sprite2: u8 = 0b00010000;
        let sprites_combined = u16::from_be_bytes([sprite1, sprite2]);

        #[rustfmt::skip]
        let state = run(&[
            // Jump past the sprites
            JP((0x200 + 4).into()).into(),
            // Sprite1 is at 0x200 + 2 and sprite2 is at 0x200 + 3
            sprites_combined,
            LDByte(r(0x1), 0x00).into(), // x coordinate to draw at
            LDByte(r(0x2), 0x00).into(), // y coordinate to draw at
            // Point I at sprite 1
            LDI((0x200 + 2).into()).into(),
            // Draw sprite1 at (V1, V2)
            DRW(r(0x1), r(0x2), 0x01).into(),
            // Point I at sprite 2
            LDI((0x200 + 3).into()).into(),
            // Draw sprite2 at (V1, V2)
            DRW(r(0x1), r(0x2), 0x01).into(),
        ]);

        // VF flips to 1 because a set pixel was changed to unset
        assert_eq!(state.get_register(0xF), 0x1);
        // These pixels stay ON
        assert_eq!(state.buffer.get_pixel(0, 0), display::ON);
        assert_eq!(state.buffer.get_pixel(1, 0), display::ON);
        assert_eq!(state.buffer.get_pixel(2, 0), display::ON);
        // This is the pixel that flipped from ON to OFF
        assert_eq!(state.buffer.get_pixel(3, 0), display::OFF);
        // These pixels stay OFF
        assert_eq!(state.buffer.get_pixel(4, 0), display::OFF);
        assert_eq!(state.buffer.get_pixel(5, 0), display::OFF);
        assert_eq!(state.buffer.get_pixel(6, 0), display::OFF);
        assert_eq!(state.buffer.get_pixel(7, 0), display::OFF);
    }

    #[test]
    fn drw_with_vf_flip_back_to_0() {
        // Draw this sprite 3 times so that VF goes from 0 -> 1 -> 0 again
        let sprite: u8 = 0b10000000;
        let sprites_combined = u16::from_be_bytes([sprite, sprite]);

        let state = run(&[
            // Jump past the sprites
            JP((0x200 + 4).into()).into(),
            // Sprite1 is at 0x200 + 2 and sprite2 is at 0x200 + 3
            sprites_combined,
            LDByte(r(0x1), 0x00).into(), // x coordinate to draw at
            LDByte(r(0x2), 0x00).into(), // y coordinate to draw at
            // Point I at sprite 1
            LDI((0x200 + 2).into()).into(),
            // Draw sprite (VF stays at 0, pixel changed from unset to set)
            DRW(r(0x1), r(0x2), 0x01).into(),
            // Draw sprite (VF flips to 1, pixel changed from set to unset)
            DRW(r(0x1), r(0x2), 0x01).into(),
            // Draw sprite (VF flips to 0, no pixel changed from set to unset)
            DRW(r(0x1), r(0x2), 0x01).into(),
        ]);

        assert_eq!(state.get_register(0xF), 0x0);
        assert_eq!(state.buffer.get_pixel(0, 0), display::ON);
        for x in 1..8 {
            assert_eq!(state.buffer.get_pixel(x, 0), display::OFF);
        }
    }

    #[test]
    fn add_registers_without_overflow() {
        let state = run(&[
            LDByte(r(0xD), 0x12).into(),
            LDByte(r(0xE), 0x20).into(),
            ADDRegister(r(0xD), r(0xE)).into(),
        ]);
        assert_eq!(state.get_register(0xD), 0x12 + 0x20);
        assert_eq!(state.get_register(0xF), 0);
    }

    #[test]
    fn add_registers_with_overflow() {
        let state = run(&[
            LDByte(r(0xD), 0x12).into(),
            LDByte(r(0xE), 0xFF).into(),
            ADDRegister(r(0xD), r(0xE)).into(),
        ]);
        assert_eq!(state.get_register(0xD), 0x11);
        assert_eq!(state.get_register(0xF), 1);
    }
}
