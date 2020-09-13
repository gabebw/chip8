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
    pub(crate) registers: Vec<u8>,
    /// A 16-bit register called I. This register is generally used to
    /// store memory addresses, so only the lowest (rightmost) 12 bits
    /// are usually used.
    i: u16,
    /// Program counter
    pub(crate) pc: u16,
    /// Stack pointer
    pub(crate) sp: u8,
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
    fn get_register<U: Into<Register>>(&mut self, unconverted: U) -> u8 {
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
                let instruction = Instruction::try_from(&chunk)?;
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
fn tick(state: &mut State, rng: Box<dyn RngCore>) -> Result<&mut State, Chip8Error> {
    let chunk = state.next_chunk().unwrap();
    // Advance by 2 bytes since 1 chunk is 2 bytes
    state.pc += 2;
    let instruction = Instruction::try_from(&chunk)?;
    execute(state, &instruction, rng, false)?;
    Ok(state)
}

/// Execute a single instruction and return the changed `State`.
fn execute<'a>(
    state: &'a mut State,
    instruction: &Instruction,
    mut rng: Box<dyn RngCore>,
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
        SE(register, byte) => {
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
        SNE(register, byte) => {
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
        LD(register, value) => {
            state.set_register(*register, *value);
            if verbosely {
                println!("\tSet register V{:X} to {:02X}", register.0, value);
            }
        }
        ADD(register, addend) => {
            let old_value = state.get_register(*register);
            state.set_register(*register, addend + old_value);
            if verbosely {
                println!(
                    "\tChanged register V{:X} from {:02X} -> {:02X}",
                    register.0,
                    old_value,
                    addend + old_value
                );
            }
        }
        ADD_REGISTERS(register_x, register_y) => {
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
    use crate::{display, instruction::Address};

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

    fn build_state_with_program(addresses_and_chunks: &[(usize, u16)]) -> State {
        State::with_program(&build_program(addresses_and_chunks))
    }

    // A random-number generator with a pre-determined seed.
    fn testing_rng() -> Box<dyn RngCore> {
        use rand::SeedableRng;
        let rng = rand::rngs::StdRng::seed_from_u64(0);
        Box::new(rng)
    }

    #[test]
    fn sys_ignored_advances_pc() {
        let mut state = build_state_with_program(&[(0, SYS().into())]);
        tick(&mut state, testing_rng()).unwrap();
        assert_eq!(state.pc, 0x202);
    }

    fn r(n: u8) -> Register {
        Register(n)
    }

    #[test]
    fn call_subroutine_and_return() {
        let mut state = build_state_with_program(&[
            // CALL: Increment SP, put current PC (0x200 + 2 = 0x202) on top of stack, set PC to 0x300
            (0, CALL(Address::unwrapped(0x300)).into()),
            // At 0x100 (+ 0x200 = 0x300 in the total program memory), do LD 1, 20
            (0x100, LD(r(0x1), 0x20).into()),
            // Now RET(urn): Set PC to top of stack (0x202) substract 1 from SP
            (0x102, RET().into()),
        ]);

        for _ in 1..=3 {
            tick(&mut state, testing_rng()).unwrap();
        }

        assert_eq!(state.pc, 0x202);
        assert_eq!(state.sp, 0);
        assert_eq!(state.get_register(0x1), 0x20);
    }

    #[test]
    fn jp_addr() {
        let mut state = build_state_with_program(&[(0, JP(Address::unwrapped(0xBCD)).into())]);
        tick(&mut state, testing_rng()).unwrap();
        assert_eq!(state.pc, 0xBCD);
    }

    #[test]
    fn ld_vx() {
        let mut state = build_state_with_program(&[(0, LD(r(0xD), 0x12).into())]);
        tick(&mut state, testing_rng()).unwrap();
        assert_eq!(state.get_register(0xD), 0x12);
    }

    #[test]
    fn ld_i() {
        let mut state = build_state_with_program(&[(0, LDI(Address::unwrapped(0x400)).into())]);
        tick(&mut state, testing_rng()).unwrap();
        assert_eq!(state.i, 0x400);
    }

    #[test]
    fn add() {
        #[rustfmt::skip]
        let mut state =
            build_state_with_program(&[
                (0, LD(r(0xD), 0x12).into()),
                (2, ADD(r(0xD), 0x12).into())
            ]);
        tick(&mut state, testing_rng()).unwrap();
        tick(&mut state, testing_rng()).unwrap();
        assert_eq!(state.get_register(0xD), 0x24);
    }

    #[test]
    fn sne() {
        let mut state = build_state_with_program(&[
            (0, LD(r(0xD), 0x12).into()),
            (2, SNE(r(0xD), 0x00).into()),
            // This should be skipped
            (4, LD(r(0x1), 0x00).into()),
            // This one should run
            (6, LD(r(0x1), 0xFF).into()),
        ]);
        for _ in 0..3 {
            tick(&mut state, testing_rng()).unwrap();
        }
        assert_eq!(state.get_register(0x1), 0xFF);
    }

    #[test]
    fn se() {
        let mut state = build_state_with_program(&[
            (0, LD(r(0xD), 0x12).into()),
            (2, SE(r(0xD), 0x12).into()),
            // This should be skipped
            (4, LD(r(0x1), 0x00).into()),
            // This one should run
            (6, LD(r(0x1), 0xFF).into()),
        ]);
        for _ in 0..3 {
            tick(&mut state, testing_rng()).unwrap();
        }
        assert_eq!(state.get_register(0x1), 0xFF);
    }

    #[test]
    fn rnd() {
        #[rustfmt::skip]
        let mut state = build_state_with_program(&[
            (0, LD(r(0x1), 0x00).into()),
            (2, RND(r(0x1), 0xFF).into()),
        ]);
        tick(&mut state, testing_rng()).unwrap();
        tick(&mut state, testing_rng()).unwrap();
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
        let mut state = build_state_with_program(&[
            (0, LD(r(0x1), 0x00).into()),
            (2, LD(r(0x2), 0x00).into()),
            // Offset by 0x200 so we're indexing into program memory
            (4, LDI(Address::unwrapped(0x200 + 12)).into()),
            // Draw sprite1
            (6, DRW(r(0x1), r(0x2), 0x01).into()),
            // Offset by 0x200 so we're indexing into program memory
            (8, LDI(Address::unwrapped(0x200 + 13)).into()),
            // Draw sprite2
            (10, DRW(r(0x1), r(0x2), 0x01).into()),
            // Sprite1 is at 12 and sprite2 is at 13
            (12, sprites_combined)
        ]);
        for _ in 0..6 {
            tick(&mut state, testing_rng()).unwrap();
        }

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

        #[rustfmt::skip]
        let mut state = build_state_with_program(&[
            (0, LD(r(0x1), 0x00).into()),
            (2, LD(r(0x2), 0x00).into()),
            // Offset by 0x200 so we're indexing into program memory
            (4, LDI(Address::unwrapped(0x200 + 12)).into()),
            // Draw sprite (VF stays at 0, pixel changed from unset to set)
            (6, DRW(r(0x1), r(0x2), 0x01).into()),
            // Draw sprite (VF flips to 1, pixel changed from set to unset)
            (8, DRW(r(0x1), r(0x2), 0x01).into()),
            // Draw sprite (VF flips to 0, no pixel changed from set to unset)
            (10, DRW(r(0x1), r(0x2), 0x01).into()),
            (12, sprites_combined)
        ]);
        for _ in 0..6 {
            tick(&mut state, testing_rng()).unwrap();
        }

        assert_eq!(state.get_register(0xF), 0x0);
        assert_eq!(state.buffer.get_pixel(0, 0), display::ON);
        for x in 1..8 {
            assert_eq!(state.buffer.get_pixel(x, 0), display::OFF);
        }
    }

    #[test]
    fn add_registers_without_overflow() {
        let mut state = build_state_with_program(&[
            (0, LD(r(0xD), 0x12).into()),
            (2, LD(r(0xE), 0x20).into()),
            (4, ADD_REGISTERS(r(0xD), r(0xE)).into()),
        ]);
        for _ in 0..3 {
            tick(&mut state, testing_rng()).unwrap();
        }
        assert_eq!(state.get_register(0xD), 0x12 + 0x20);
        assert_eq!(state.get_register(0xF), 0);
    }

    #[test]
    fn add_registers_with_overflow() {
        let mut state = build_state_with_program(&[
            (0, LD(r(0xD), 0x12).into()),
            (2, LD(r(0xE), 0xFF).into()),
            (4, ADD_REGISTERS(r(0xD), r(0xE)).into()),
        ]);
        for _ in 0..3 {
            tick(&mut state, testing_rng()).unwrap();
        }
        assert_eq!(state.get_register(0xD), 0x11);
        assert_eq!(state.get_register(0xF), 1);
    }
}
