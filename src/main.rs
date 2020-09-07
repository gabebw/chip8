// Allow dead code for now, as it's being built.
#![allow(dead_code)]

mod cli;
mod instruction;
mod interpreter;
mod parser;

use cli::Subcommand::*;
use instruction::Instruction;
use interpreter::State;
use itertools::Itertools;
use std::error::Error;
use std::{
    convert::TryInto,
    fs::File,
    io::{BufReader, Read},
};
use structopt::StructOpt;

fn read_be_u16(input: &mut &[u8]) -> u16 {
    let (int_bytes, rest) = input.split_at(std::mem::size_of::<u16>());
    *input = rest;
    u16::from_be_bytes(int_bytes.try_into().unwrap())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = cli::Arguments::from_args();
    match options.subcommand {
        Print { input_file_path } => {
            let file = BufReader::new(File::open(input_file_path)?);
            let contents = file.bytes().collect::<Result<Vec<u8>, std::io::Error>>()?;
            for multibytes in &contents.into_iter().chunks(2) {
                let bytes = read_be_u16(&mut multibytes.collect::<Vec<u8>>().as_slice());
                let instruction: Instruction = (&bytes).try_into()?;
                println!("{:04X} => {}", bytes, instruction);
            }
        }
    };
    Ok(())
}

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
