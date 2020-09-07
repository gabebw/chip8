use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Given the value 0xABCD, return 0xBCD.
/// A nibble is a 12-bit value.
fn nibble(bytes: &u16) -> u16 {
    bytes & 0x0FFF
}

#[derive(Debug, PartialEq, Copy, Clone)]
/// An Address is a 12-bit nibble stored in a u16.
pub struct Address(u16);

impl Address {
    pub fn unwrapped(chunk: u16) -> Self {
        Self::try_from(chunk).unwrap()
    }
}

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

#[derive(Debug, PartialEq)]
pub enum Instruction {
    /// Ignored
    SYS(),

    /// Return from a subroutine.
    /// The interpreter sets the program counter to the address at the top of the
    /// stack, then subtracts 1 from the stack pointer.
    RET(),

    // Jump to location nnn. The interpreter sets the program counter to nnn.
    JP(Address),

    /// Set Vx = kk. The interpreter puts the value kk into register Vx.
    LD(u8, u8),

    /// Call subroutine at nnn.
    /// The interpreter increments the stack pointer, then puts the current PC on
    /// the top of the stack. The PC is then set to nnn.
    CALL(Address),

    /// Set register I to nnn.
    LDI(Address),

    /// Until this program knows how to parse every CHIP-8 instruction, this
    /// makes it possible to print out "unknown" (so far) instructions.
    UNKNOWN(u16),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Instruction::*;

        match self {
            SYS() => write!(f, "SYS (ignored)"),
            RET() => write!(f, "RET"),
            JP(address) => write!(f, "JP {:02X}", address.0),
            LD(register, value) => write!(f, "LD {:X}, {:02X}", register, value),
            CALL(address) => write!(f, "CALL {:02X}", address.0),
            LDI(address) => write!(f, "LD I, {:02X}", address.0),
            UNKNOWN(bytes) => write!(f, "Unknown: {:02X}", bytes),
        }
    }
}

fn address(chunk: &u16) -> Result<Address, Box<dyn Error>> {
    nibble(chunk).try_into()
}

impl TryFrom<&u16> for Instruction {
    type Error = Box<dyn Error>;
    fn try_from(chunk: &u16) -> Result<Self, Self::Error> {
        use Instruction::*;
        // Convert a 2-byte value in the format 0xABCD into 0xA and 0xB
        let a: u8 = (chunk >> 12).try_into()?;
        let b: u8 = (chunk >> 8 & 0x000F).try_into()?;

        let instruction = match a {
            0x0 => match chunk {
                0x00EE => RET(),
                _ => SYS(),
            },
            0x1 => JP(address(&chunk)?),
            0x2 => CALL(address(&chunk)?),
            0x6 => {
                let value: u8 = (chunk & 0xFF).try_into()?;
                LD(b, value)
            }
            0xA => LDI(address(&chunk)?),
            _ => UNKNOWN(*chunk),
        };
        Ok(instruction)
    }
}
