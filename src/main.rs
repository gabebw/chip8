#[macro_use]
extern crate log;

mod cli;
mod display;
mod error;
mod instruction;
mod interpreter;

use cli::Subcommand::*;
use error::Chip8Error;
use instruction::Instruction;
use interpreter::State;
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

fn main() -> Result<(), Chip8Error> {
    let options = cli::Arguments::from_args();
    let mut verbose = options.verbose;
    cli::install_logger(&mut verbose);

    match options.subcommand {
        Print { input_file_path } => {
            let file = BufReader::new(File::open(input_file_path)?);
            let contents = file.bytes().collect::<Result<Vec<u8>, std::io::Error>>()?;
            for mut multibytes in contents.as_slice().chunks_exact(2) {
                let bytes = read_be_u16(&mut multibytes);
                let instruction: Instruction = (&bytes).try_into()?;
                println!("{:04X} => {}", bytes, instruction);
            }
        }
        Trace { input_file_path } => {
            let file = BufReader::new(File::open(input_file_path)?);
            let contents = file.bytes().collect::<Result<Vec<u8>, std::io::Error>>()?;
            let mut state = State::with_program(&contents);
            interpreter::run(&mut state, true)?;
        }
    };
    Ok(())
}
