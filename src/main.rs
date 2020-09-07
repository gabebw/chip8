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
        Trace { input_file_path } => {
            let file = BufReader::new(File::open(input_file_path)?);
            let contents = file.bytes().collect::<Result<Vec<u8>, std::io::Error>>()?;
            let mut state = State::with_program(&contents);
            interpreter::run(&mut state, true)?;
        }
    };
    Ok(())
}
