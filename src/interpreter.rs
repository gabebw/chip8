use crate::error::Chip8Error;
use crate::{
    display::{Display, ScaledFramebuffer},
    instruction::{Instruction, Instruction::*},
};
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
    /// Create a new State with default values.
    pub fn new() -> Self {
        Self {
            memory: vec![0; 4096],
            registers: vec![0; 16],
            i: 0,
            pc: 0x200,
            sp: 0,
            stack: vec![0; 16],
            buffer: ScaledFramebuffer::new(),
        }
    }

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
    fn set_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = value;
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
        let value = self.stack[self.sp as usize];
        value
    }

    fn next_chunk(&self) -> Option<u16> {
        let one = self.memory.get(self.pc as usize)?;
        let two = self.memory.get((self.pc + 1) as usize)?;
        Some(u16::from_be_bytes([*one, *two]))
    }
}

/// Run the entire program, forever.
pub fn run<'a>(state: &'a mut State, verbosely: bool) -> Result<&'a mut State, Chip8Error> {
    let mut display = Display::new(state.buffer.true_width, state.buffer.true_height);
    while display.is_running() {
        match state.next_chunk() {
            Some(chunk) => {
                // Advance by 2 bytes since 1 chunk is 2 bytes
                state.pc += 2;
                let instruction = Instruction::try_from(&chunk)?;
                execute(state, &instruction, verbosely)?;
                display.draw(&mut state.buffer);

                if let DRW(_, _, _) = instruction {
                    // Show the thing we just drew because otherwise it
                    // disappears immediately when the program panics on an
                    // instruction it doesn't understand
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            }
            None => break,
        }
    }
    Ok(state)
}

// Do one thing in the interpreter (run one instruction) and return the changed state.
// Useful for testing.
fn tick<'a>(state: &'a mut State) -> Result<&'a mut State, Chip8Error> {
    let chunk = state.next_chunk().unwrap();
    // Advance by 2 bytes since 1 chunk is 2 bytes
    state.pc += 2;
    let instruction = Instruction::try_from(&chunk)?;
    execute(state, &instruction, false)?;
    Ok(state)
}

/// Execute a single instruction and return the changed `State`.
fn execute<'a>(
    state: &'a mut State,
    instruction: &Instruction,
    verbosely: bool,
) -> Result<&'a mut State, Chip8Error> {
    if verbosely {
        println!("{}", instruction);
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
        LD(register, value) => {
            state.set_register(*register, *value);
            if verbosely {
                println!("\tSet register {:04X} to {:04X}", register, value);
            }
        }
        LDI(address) => {
            let value = (*address).into();
            state.i = value;
            if verbosely {
                println!("\tSet register I to {:04X}", value);
            }
        }
        DRW(x, y, n) => {
            let slice_start = state.i as usize;
            let slice_end = slice_start + (*n as usize);
            let sprite = &state.memory[slice_start..slice_end];
            state
                .buffer
                .draw_sprite_at(*x as usize, *y as usize, sprite);
            if verbosely {
                let pretty_sprite = sprite
                    .iter()
                    .map(|byte| format!("\t{:08b}", byte))
                    .collect::<Vec<_>>()
                    .join("\n");
                println!("\tSprite data:\n{}", pretty_sprite);
            }
        }
        UNKNOWN(bytes) => {
            panic!("Unknown instruction: {:04X}", bytes);
        }
    }
    Ok(state)
}

mod test {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::instruction::Address;

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

    #[test]
    fn sys_ignored_advances_pc() {
        let mut state = build_state_with_program(&[(0, 0x0000)]);
        tick(&mut state).unwrap();
        assert_eq!(state.pc, 0x202);
    }

    #[test]
    fn call_subroutine_and_return() {
        let mut state = build_state_with_program(&[
            // CALL: Increment SP, put current PC (0x200 + 2 = 0x202) on top of stack, set PC to 0x300
            (0, 0x2300),
            // At 0x100 (+ 0x200 = 0x300 in the total program memory), do LD 1, 20
            (0x100, 0x6120),
            // Now RET(urn): Set PC to top of stack (0x202) substract 1 from SP
            (0x102, 0x00EE),
        ]);

        for _ in 1..=3 {
            tick(&mut state).unwrap();
        }

        assert_eq!(state.pc, 0x202);
        assert_eq!(state.sp, 0);
        assert_eq!(state.registers.get(0x1).copied().unwrap(), 0x20);
    }

    #[test]
    fn jp_addr() {
        let mut state = build_state_with_program(&[(0, 0x1BCD)]);
        tick(&mut state).unwrap();
        assert_eq!(state.pc, 0xBCD);
    }

    #[test]
    fn ld_vx() {
        let mut state = build_state_with_program(&[(0, 0x6D12)]);
        tick(&mut state).unwrap();
        assert_eq!(state.registers.get(0xD).copied().unwrap(), 0x12);
    }

    #[test]
    fn ld_i() {
        let mut state = build_state_with_program(&[(0, 0xA400)]);
        tick(&mut state).unwrap();
        assert_eq!(state.i, 0x400);
    }
}
