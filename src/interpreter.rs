use crate::instruction::{Instruction, Instruction::*};
use std::{convert::TryInto, error::Error};

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
}

impl State {
    /// Create a new State with default values.
    pub fn new() -> Self {
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

    /// Increment the stack pointer and push a value onto the top of the stack.
    fn push_onto_stack(&mut self, value: u16) {
        self.sp += 1;
        self.stack[self.sp as usize] = value;
    }

    /// Decrement the stack pointer and return the value that it used to point to.
    fn pop_off_stack(&mut self) -> u16 {
        if self.sp == 0 {
            panic!("Cannot decrement stack pointer, already at 0");
        }
        let value = self.stack[self.sp as usize];
        self.sp -= 1;
        value
    }
}

pub fn run<'a>(
    state: &'a mut State,
    instructions: &[Instruction],
) -> Result<&'a mut State, Box<dyn Error>> {
    for instruction in instructions {
        match instruction {
            SYS() => {
                // Ignore it and do nothing
            }
            RET() => state.pc = state.pop_off_stack(),
            JP(address) => {
                state.set_pc((*address).try_into()?);
            }
            CALL(address) => {
                state.push_onto_stack(state.pc);
                state.set_pc((*address).try_into()?);
            }
            LD(register, value) => {
                // Set Vx = kk.
                // The interpreter puts the value kk into register Vx.
                state.set_register(*register, *value);
            }
        }
    }

    Ok(state)
}

mod test {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::instruction::Address;

    fn run_program(program: Vec<Instruction>) -> State {
        let mut state = State::new();
        run(&mut state, &program).unwrap().to_owned()
    }

    #[test]
    fn sys_ignored() {
        let mut state = State::new();
        let program = vec![SYS()];
        let new_state = run(&mut state, &program).unwrap().clone();
        assert_eq!(state, new_state);
    }

    #[test]
    fn call_subroutine_and_return() {
        let program = vec![
            JP(Address::unwrapped(0xABC)),   // Set PC to 0xABC
            CALL(Address::unwrapped(0xBCD)), // Increment SP, put current PC on top of stack, set PC to BCD
            CALL(Address::unwrapped(0xDEF)), // Increment SP, put current PC on top of stack, set PC to DEF
            RET(),                           // Set PC to top of stack (BCD), substract 1 from SP
        ];
        let new_state = run_program(program);
        assert_eq!(new_state.pc, 0xBCD);
        assert_eq!(new_state.sp, 1);
    }

    #[test]
    fn jp_addr() {
        let new_state = run_program(vec![JP(Address::unwrapped(0xBCD))]);
        assert_eq!(new_state.pc, 0xBCD);
    }

    #[test]
    fn ld_vx() {
        let new_state = run_program(vec![LD(0xD, 0x12)]);
        assert_eq!(new_state.registers.get(0xD).copied().unwrap(), 0x12);
    }
}
