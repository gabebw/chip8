// Allow dead code for now, as it's being built.
#![allow(dead_code)]

use std::convert::TryInto;

struct State {
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
}

impl State {
    /// Create a new State with default values.
    fn new() -> Self {
        Self {
            memory: vec![0; 4096],
            registers: vec![0; 16],
            i: 0,
            pc: 0,
            sp: 0,
            stack: vec![0; 16],
        }
    }

    /// Set the given register to the given value.
    fn set_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = value;
    }

    /// Set the program counter to the given address.
    /// The address is a 12-bit nibble, meaning that its maximum value is 0x0FFF,
    /// not 0xFFFF as the u16 type implies.
    fn set_pc(&mut self, address: u16) {
        assert!(address <= 0xFFF);
        self.pc = address;
    }
}

/// Given the value 0xABCD, return 0xBCD.
/// A nibble is a 12-bit value.
fn nibble(bytes: &u16) -> u16 {
    bytes & 0x0FFF
}

fn run<'a>(
    state: &'a mut State,
    program: &[u16],
) -> Result<&'a mut State, Box<dyn std::error::Error>> {
    for chunk in program {
        // Convert a 2-byte value in the format 0xABCD into 0xA and 0xB
        let a: u8 = (chunk >> 12).try_into()?;
        let b: u8 = (chunk >> 8 & 0x000F).try_into()?;

        match a {
            // 1nnn - JP addr
            // Jump to location nnn.
            // The interpreter sets the program counter to nnn.
            0x1 => {
                let nibble = nibble(chunk);
                state.set_pc(nibble);
            }
            // 6xkk - LD Vx, byte
            // Set Vx = kk.
            // The interpreter puts the value kk into register Vx.
            0x6 => {
                let value: u8 = (chunk & 0xFF).try_into()?;
                state.set_register(b, value);
            }
            _ => panic!("Instruction not supported: {:x}", chunk),
        }
    }

    Ok(state)
}

fn main() {}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn jp_addr() {
        let mut state = State::new();
        let program = vec![0x1BCD];
        let new_state = run(&mut state, &program).unwrap();
        assert_eq!(new_state.pc, 0xBCD);
    }

    #[test]
    fn ld_vx() {
        let mut state = State::new();
        let program = vec![0x6D12];
        let new_state = run(&mut state, &program).unwrap();
        assert_eq!(new_state.registers.get(0xD).copied().unwrap(), 0x12);
    }
}
