// Allow dead code for now, as it's being built.
#![allow(dead_code)]

mod instruction;
mod interpreter;
mod parser;

use interpreter::State;

fn main() {}

mod test {
    #[allow(unused_imports)]
    use super::*;

    fn run_program(program: Vec<u16>) -> State {
        let mut state = State::new();
        interpreter::run(&mut state, &parser::parse(&program).unwrap())
            .unwrap()
            .to_owned()
    }

    #[test]
    fn sys_ignored() {
        let program = vec![0x0ABC];
        let new_state = run_program(program);
        assert_eq!(State::new(), new_state);
    }

    #[test]
    fn call_subroutine_and_return() {
        let program = vec![
            0x1ABC, // Set PC to 0xABC
            0x2BCD, // Increment SP, put current PC on top of stack, set PC to BCD
            0x2DEF, // Increment SP, put current PC on top of stack, set PC to DEF
            0x00EE, // Set PC to top of stack (BCD), substract 1 from SP
        ];
        let new_state = run_program(program);
        assert_eq!(new_state.pc, 0xBCD);
        assert_eq!(new_state.sp, 1);
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
