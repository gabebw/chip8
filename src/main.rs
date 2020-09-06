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

    fn set_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = value;
    }
}

fn run<'a>(
    state: &'a mut State,
    program: &[u16],
) -> Result<&'a mut State, Box<dyn std::error::Error>> {
    for byte in program {
        // Convert a 2-byte value in the format 0xABCD into 0xA and 0xB
        let a: u8 = (byte >> 12).try_into()?;
        let b: u8 = (byte >> 8 & 0x000F).try_into()?;

        match a {
            0x6 => {
                let value: u8 = (byte & 0xFF).try_into()?;
                state.set_register(b, value);
            }
            _ => panic!("Instruction not supported: {:x}", byte),
        }
    }

    Ok(state)
}

fn main() {}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    // 6xkk - LD Vx, byte
    // Set Vx = kk.
    fn ld_vx() {
        let mut state = State::new();
        let program = vec![0x6D12];
        let new_state = run(&mut state, &program).unwrap();
        assert_eq!(new_state.registers.get(0xD).copied().unwrap(), 0x12);
    }
}
