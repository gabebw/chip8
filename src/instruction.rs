use std::convert::{TryFrom, TryInto};
use std::error::Error;

/// Given the value 0xABCD, return 0xBCD.
/// A nibble is a 12-bit value.
fn nibble(bytes: &u16) -> u16 {
    bytes & 0x0FFF
}

#[derive(Debug)]
/// An Address is a 12-bit nibble stored in a u16.
pub struct Address(u16);

impl TryFrom<u16> for Address {
    type Error = Box<dyn Error>;

    /// The address is a 12-bit nibble, meaning that its maximum value is 0x0FFF,
    /// not 0xFFFF as the u16 type implies.
    /// It will panic if passed a value larger than 0x0FFF.
    fn try_from(chunk: u16) -> Result<Self, Self::Error> {
        assert!(chunk <= 0x0FFF);
        Ok(Self(chunk))
    }
}

impl Into<u16> for Address {
    fn into(self) -> u16 {
        self.0
    }
}

impl PartialEq<u16> for Address {
    fn eq(&self, number: &u16) -> bool {
        self.0 == *number
    }
}

#[derive(Debug)]
pub enum Instruction {
    JP(Address),
    LD(u8, u8),
}

impl TryFrom<&u16> for Instruction {
    type Error = Box<dyn Error>;
    fn try_from(chunk: &u16) -> Result<Self, Self::Error> {
        use Instruction::*;
        // Convert a 2-byte value in the format 0xABCD into 0xA and 0xB
        let a: u8 = (chunk >> 12).try_into()?;
        let b: u8 = (chunk >> 8 & 0x000F).try_into()?;

        match a {
            // 1nnn - JP addr
            // Jump to location nnn. The interpreter sets the program counter to
            // nnn.
            0x1 => Ok(JP(nibble(&chunk).try_into()?)),
            // 6xkk - LD Vx, byte
            // Set Vx = kk. The interpreter puts the value kk into register Vx.
            0x6 => {
                let value: u8 = (chunk & 0xFF).try_into()?;
                Ok(LD(b, value))
            }
            _ => panic!("Instruction not supported: {:x}", chunk),
        }
    }
}
