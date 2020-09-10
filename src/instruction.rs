use crate::error::Chip8Error;
use std::convert::{TryFrom, TryInto};
use std::fmt::{Display, Formatter};

/// Given the value 0xABCD, return 0xBCD.
fn last_3_bits(bytes: &u16) -> u16 {
    bytes & 0x0FFF
}

#[derive(Debug, PartialEq, Copy, Clone)]
/// An Address is a 12-bit value stored in a u16.
pub struct Address(u16);

impl Address {
    pub fn unwrapped(chunk: u16) -> Self {
        Self::try_from(chunk).unwrap()
    }
}

impl TryFrom<u16> for Address {
    type Error = Chip8Error;

    /// The address is a 12-bit value, meaning that its maximum value is 0x0FFF,
    /// not 0xFFFF as the u16 type implies.
    /// It will panic if passed a value larger than 0x0FFF.
    fn try_from(chunk: u16) -> Result<Self, Self::Error> {
        if chunk > 0x0FFF {
            Err(Chip8Error::NibbleTooLarge(chunk))
        } else {
            Ok(Self(chunk))
        }
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

    /// Call subroutine at nnn.
    /// The interpreter increments the stack pointer, then puts the current PC on
    /// the top of the stack. The PC is then set to nnn.
    CALL(Address),

    /// Set Vx = kk. The interpreter puts the value kk into register Vx.
    LD(u8, u8),

    /// Vx += kk
    /// Adds the value kk to the value of register Vx, then stores the result in Vx.
    ADD(u8, u8),

    /// Set register I to nnn.
    LDI(Address),

    /// DRW x, y, n
    /// Display n-byte sprite starting at memory location I at (Vx, Vy).
    DRW(u8, u8, u8),

    // ADD I, Vx
    // Set I = I + Vx.
    ADDI(u8),

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
            CALL(address) => write!(f, "CALL {:02X}", address.0),
            LD(register, value) => write!(f, "LD V{:X}, {:02X}", register, value),
            ADD(register, addend) => write!(f, "ADD V{:X}, {:02X}", register, addend),
            LDI(address) => write!(f, "LD I, {:02X}", address.0),
            DRW(x, y, n) => write!(f, "DRW V{:X}, V{:X}, {:02X}", x, y, n),
            ADDI(register) => write!(f, "ADD I, V{:X}", register),
            UNKNOWN(bytes) => write!(f, "Unknown: {:02X}", bytes),
        }
    }
}

fn address(chunk: &u16) -> Result<Address, Chip8Error> {
    last_3_bits(chunk).try_into()
}

/// Break a u8 like 0xAB into 0xA and 0xB
fn nibbles(byte: u8) -> [u8; 2] {
    let a: u8 = byte >> 4;
    let b: u8 = byte & 0x0F;
    [a, b]
}

impl TryFrom<&u16> for Instruction {
    type Error = Chip8Error;

    fn try_from(chunk: &u16) -> Result<Self, Self::Error> {
        use Instruction::*;
        // Convert a 2-byte value in the format 0xABCD into 0xA, 0xB, 0xC, and 0xD
        let [byte1, byte2] = u16::to_be_bytes(*chunk);
        let [a, b] = nibbles(byte1);
        let [c, d] = nibbles(byte2);

        let instruction = match a {
            0x0 => match chunk {
                0x00EE => RET(),
                _ => SYS(),
            },
            0x1 => JP(address(&chunk)?),
            0x2 => CALL(address(&chunk)?),
            0x6 => LD(b, byte2),
            0x7 => ADD(b, byte2),
            0xA => LDI(address(&chunk)?),
            0xD => DRW(b, c, d),
            0xF => ADDI(b),
            _ => UNKNOWN(*chunk),
        };
        Ok(instruction)
    }
}
