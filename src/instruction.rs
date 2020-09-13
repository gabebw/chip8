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
    #[cfg(test)]
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

#[derive(Debug, PartialEq, Clone)]
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

    /// Skip next instruction if Vx == kk.
    SE(u8, u8),

    /// Skip next instruction if Vx != kk.
    SNE(u8, u8),

    /// Set Vx = kk. The interpreter puts the value kk into register Vx.
    LD(u8, u8),

    /// Vx += kk
    /// Adds the value kk to the value of register Vx, then stores the result in Vx.
    ADD(u8, u8),

    /// Set register I to nnn.
    LDI(Address),

    /// Set Vx = random byte & kk.
    RND(u8, u8),

    /// DRW Vx, Vy, n
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
            SE(register, byte) => write!(f, "SE V{:X}, {:02X}", register, byte),
            SNE(register, byte) => write!(f, "SNE V{:X}, {:02X}", register, byte),
            LD(register, value) => write!(f, "LD V{:X}, {:02X}", register, value),
            ADD(register, addend) => write!(f, "ADD V{:X}, {:02X}", register, addend),
            LDI(address) => write!(f, "LD I, {:02X}", address.0),
            RND(register, byte) => write!(f, "RND V{:X}, {:02X}", register, byte),
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
            0x3 => SE(b, byte2),
            0x4 => SNE(b, byte2),
            0x6 => LD(b, byte2),
            0x7 => ADD(b, byte2),
            0xA => LDI(address(&chunk)?),
            0xC => RND(b, byte2),
            0xD => DRW(b, c, d),
            0xF => ADDI(b),
            _ => UNKNOWN(*chunk),
        };
        Ok(instruction)
    }
}

impl Into<u16> for Instruction {
    fn into(self) -> u16 {
        use Instruction::*;

        // Yes, it's not actually tens/hundreds/thousands places since we're in
        // hexadecimal, but it's a helpful idea.
        let tens = |n: u8| u16::from(n) * 0x10;
        let hundreds = |n: u8| u16::from(n) * 0x100;

        match self {
            // Since SYS is technically any 0nnn opcode that's not 00E0 or 00EE,
            // just pick something that's not used by anything else.
            SYS() => 0x0123,
            RET() => 0x00EE,
            JP(address) => 0x1000 + address.0,
            CALL(address) => 0x2000 + address.0,
            SE(register, byte) => 0x3000 + hundreds(register) + u16::from(byte),
            SNE(register, byte) => 0x4000 + hundreds(register) + u16::from(byte),
            LD(register, byte) => 0x6000 + hundreds(register) + u16::from(byte),
            ADD(register, byte) => 0x7000 + hundreds(register) + u16::from(byte),
            LDI(address) => 0xA000 + address.0,
            RND(register, byte) => 0xC000 + hundreds(register) + u16::from(byte),
            DRW(x, y, n) => 0xD000 + hundreds(x) + tens(y) + u16::from(n),
            ADDI(register) => 0xF000 + hundreds(register) + 0x1E,
            UNKNOWN(bytes) => bytes,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Instruction::*, *};

    // This helper function exists so that we don't have to inline an ugly
    // `Into::<u16>::into(instruction)` into all the other tests. The return
    // type on this function gives `into()` the information it needs.
    fn into_u16(i: Instruction) -> u16 {
        i.into()
    }

    #[test]
    fn as_u16_ret() {
        assert_eq!(into_u16(RET()), 0x00EE)
    }

    #[test]
    fn as_u16_sys() {
        assert_eq!(into_u16(SYS()), 0x0123)
    }

    #[test]
    fn as_u16_jp() {
        assert_eq!(into_u16(JP(Address::unwrapped(0x234))), 0x1234)
    }

    #[test]
    fn as_u16_call() {
        assert_eq!(into_u16(CALL(Address::unwrapped(0x345))), 0x2345)
    }

    #[test]
    fn as_u16_se() {
        assert_eq!(into_u16(SE(0x4, 0x56)), 0x3456)
    }

    #[test]
    fn as_u16_sne() {
        assert_eq!(into_u16(SNE(0x5, 0x67)), 0x4567)
    }

    #[test]
    fn as_u16_ld() {
        assert_eq!(into_u16(LD(0x7, 0x89)), 0x6789);
    }

    #[test]
    fn as_u16_add() {
        assert_eq!(into_u16(ADD(0x8, 0x9A)), 0x789A)
    }

    #[test]
    fn as_u16_ldi() {
        assert_eq!(into_u16(LDI(Address::unwrapped(0xBCD))), 0xABCD)
    }

    #[test]
    fn as_u16_drw() {
        assert_eq!(into_u16(DRW(0xA, 0xB, 0xC)), 0xDABC)
    }

    #[test]
    fn as_u16_addi() {
        assert_eq!(into_u16(ADDI(0xB)), 0xFB1E)
    }

    #[test]
    fn as_u16_rnd() {
        assert_eq!(into_u16(RND(0xA, 0xBC)), 0xCABC)
    }

    #[test]
    fn from_u16() {
        use std::collections::HashMap;

        #[rustfmt::skip]
        let instructions: HashMap<u16, Instruction> = [
            (0x00EE, RET()),
            (0x0ABC, SYS()),
            (0x1A12, JP(0xA12.try_into().unwrap())),
            (0x221A, CALL(0x21A.try_into().unwrap())),
            (0x3934, SE(0x9, 0x34)),
            (0x4A56, SNE(0xA, 0x56)),
            (0x6003, LD(0x0, 0x03.try_into().unwrap())),
            (0x7123, ADD(0x1, 0x23)),
            (0xA278, LDI(0x278.try_into().unwrap())),
            (0xC123, RND(0x1, 0x23)),
            (0xD123, DRW(0x1, 0x2, 0x3)),
            (0xF51E, ADDI(0x5))
        ].iter().cloned().collect();

        for (chunk, instruction) in instructions.into_iter() {
            let actual = Instruction::try_from(&chunk).unwrap();
            assert_eq!(actual, instruction);
        }
    }
}
