// Allow dead code for now, as it's being built.
#![allow(dead_code)]

mod instruction;

use instruction::{Instruction, Instruction::*};
use std::convert::TryFrom;
use std::error::Error;

#[derive(Clone, Debug)]
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
    fn set_pc(&mut self, address: u16) {
        self.pc = address;
    }
}

fn run<'a>(state: &'a mut State, program: &[u16]) -> Result<&'a mut State, Box<dyn Error>> {
    let instructions: Vec<Instruction> = program
        .iter()
        .map(Instruction::try_from)
        .collect::<Result<Vec<Instruction>, Box<dyn Error>>>()?;
    for instruction in instructions {
        match instruction {
            JP(address) => {
                // Jump to location nnn.
                // The interpreter sets the program counter to nnn.
                state.set_pc(address.into());
            }
            LD(register, value) => {
                // Set Vx = kk.
                // The interpreter puts the value kk into register Vx.
                state.set_register(register, value);
            }
        }
    }

    Ok(state)
}

fn main() {}

mod test {
    #[allow(unused_imports)]
    use super::*;

    fn run_program(program: Vec<u16>) -> State {
        let mut state = State::new();
        run(&mut state, &program).unwrap().to_owned()
    }

    #[test]
    fn jp_addr() {
        let new_state = run_program(vec![0x1BCD]);
        assert_eq!(new_state.pc, 0xBCD);
    }

    #[test]
    fn ld_vx() {
        let new_state = run_program(vec![0x6D12]);
        assert_eq!(new_state.registers.get(0xD).copied().unwrap(), 0x12);
    }
}
