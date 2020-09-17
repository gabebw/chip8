use crate::error::Chip8Error;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Copy, Clone)]
/// An Address is a 12-bit value stored in a u16.
pub struct Address(u16);

impl From<u16> for Address {
    /// The address is a 12-bit value, meaning that its maximum value is 0x0FFF,
    /// not 0xFFFF as the u16 type implies.
    /// If passed a value larger than 0x0FFF, it will use only the last 3 bits.
    fn from(chunk: u16) -> Self {
        Self(chunk & 0x0FFF)
    }
}

// This makes it much easier to write tests, by allowing writing `0x32.into()`
// (since `0x32` is detected as an i32 by default) rather than having to write
// this clunky phrase: `(0x32 as u16).into()`.
#[cfg(test)]
impl From<i32> for Address {
    fn from(chunk: i32) -> Self {
        use std::convert::TryInto;
        u16::try_from(chunk).unwrap().try_into().unwrap()
    }
}

impl Into<u16> for Address {
    fn into(self) -> u16 {
        self.0
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
/// A Register is a 4-bit value that addresses a register numbered from 0x0 to 0xF.
pub struct Register(pub u8);

impl From<u8> for Register {
    fn from(n: u8) -> Self {
        if n > 0xF {
            panic!("Register value must be from 0x0 - 0xF (got {:X})", n)
        }
        Register(n)
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
    SEByte(Register, u8),

    /// Skip next instruction if Vx != kk.
    SNEByte(Register, u8),

    /// Skip next instruction if Vx == Vy.
    SERegister(Register, Register),

    /// Skip next instruction if Vx == Vy.
    SNERegister(Register, Register),

    /// Set Vx = kk. The interpreter puts the value kk into register Vx.
    LDByte(Register, u8),

    /// Vx += kk
    /// Adds the value kk to the value of register Vx, then stores the result in Vx.
    ADDByte(Register, u8),

    /// Vx += Vy
    /// Set Vx = Vx + Vy, set VF = carry.
    /// The values of Vx and Vy are added together. If the result is greater than
    /// 8 bits (i.e., > 255,) VF is set to 1, otherwise 0.
    /// Only the lowest 8 bits of the result are kept, and stored in Vx.
    ADDRegister(Register, Register),

    /// Set register I to nnn.
    LDI(Address),

    /// Set Vx = random byte & kk.
    RND(Register, u8),

    /// DRW Vx, Vy, n
    /// Display n-byte sprite starting at memory location I at (Vx, Vy).
    DRW(Register, Register, u8),

    // ADD I, Vx
    // Set I = I + Vx.
    ADDI(Register),

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
            SEByte(register, byte) => write!(f, "SE V{:X}, {:02X}", register.0, byte),
            SNEByte(register, byte) => write!(f, "SNE V{:X}, {:02X}", register.0, byte),
            SERegister(register_x, register_y) => {
                write!(f, "SE V{:X}, V{:X}", register_x.0, register_y.0)
            }
            SNERegister(register_x, register_y) => {
                write!(f, "SNE V{:X}, V{:X}", register_x.0, register_y.0)
            }
            LDByte(register, byte) => write!(f, "LD V{:X}, {:02X}", register.0, byte),
            ADDByte(register, byte) => write!(f, "ADD V{:X}, {:02X}", register.0, byte),
            ADDRegister(register_x, register_y) => {
                write!(f, "ADD V{:X} += V{:X}", register_x.0, register_y.0)
            }
            LDI(address) => write!(f, "LD I, {:02X}", address.0),
            RND(register, byte) => write!(f, "RND V{:X}, {:02X}", register.0, byte),
            DRW(x, y, n) => write!(f, "DRW V{:X}, V{:X}, {:02X}", x.0, y.0, n),
            ADDI(register) => write!(f, "ADD I, V{:X}", register.0),
            UNKNOWN(bytes) => write!(f, "Unknown: {:02X}", bytes),
        }
    }
}

/// Break a u8 like 0xAB into 0xA and 0xB
fn nibbles(byte: u8) -> [u8; 2] {
    let a: u8 = byte >> 4;
    let b: u8 = byte & 0x0F;
    [a, b]
}

impl TryFrom<u16> for Instruction {
    type Error = Chip8Error;

    fn try_from(chunk: u16) -> Result<Self, Self::Error> {
        use Instruction::*;
        // Convert a 2-byte value in the format 0xABCD into 0xA, 0xB, 0xC, and 0xD
        let [byte1, byte2] = u16::to_be_bytes(chunk);
        let [a, b] = nibbles(byte1);
        let [c, d] = nibbles(byte2);

        let instruction = match a {
            0x0 => match chunk {
                0x00EE => RET(),
                _ => SYS(),
            },
            0x1 => JP(chunk.into()),
            0x2 => CALL(chunk.into()),
            0x3 => SEByte(Register(b), byte2),
            0x4 => SNEByte(Register(b), byte2),
            0x5 => {
                if d == 0 {
                    // Chunk is 5bc0
                    SERegister(Register(b), Register(c))
                } else {
                    UNKNOWN(chunk)
                }
            }
            0x6 => LDByte(Register(b), byte2),
            0x7 => ADDByte(Register(b), byte2),
            0x8 => ADDRegister(Register(b), Register(c)),
            0x9 => {
                if d == 0 {
                    // Chunk is 9bc0
                    SNERegister(Register(b), Register(c))
                } else {
                    UNKNOWN(chunk)
                }
            }
            0xA => LDI(chunk.into()),
            0xC => RND(Register(b), byte2),
            0xD => DRW(Register(b), Register(c), d),
            0xF => ADDI(Register(b)),
            _ => UNKNOWN(chunk),
        };
        Ok(instruction)
    }
}

impl Into<u16> for Instruction {
    fn into(self) -> u16 {
        use Instruction::*;

        // Yes, it's not actually tens/hundreds/thousands places since we're in
        // hexadecimal, but it's a helpful idea.
        let tens = |n: Register| u16::from(n.0) * 0x10;
        let hundreds = |n: Register| u16::from(n.0) * 0x100;

        match self {
            // Since SYS is technically any 0nnn opcode that's not 00E0 or 00EE,
            // just pick something that's not used by anything else.
            SYS() => 0x0123,
            RET() => 0x00EE,
            JP(address) => 0x1000 + address.0,
            CALL(address) => 0x2000 + address.0,
            SEByte(register, byte) => 0x3000 + hundreds(register) + u16::from(byte),
            SNEByte(register, byte) => 0x4000 + hundreds(register) + u16::from(byte),
            SERegister(register_x, register_y) => 0x5000 + hundreds(register_x) + tens(register_y),
            SNERegister(register_x, register_y) => 0x9000 + hundreds(register_x) + tens(register_y),
            LDByte(register, byte) => 0x6000 + hundreds(register) + u16::from(byte),
            ADDByte(register, byte) => 0x7000 + hundreds(register) + u16::from(byte),
            ADDRegister(register_x, register_y) => {
                0x8000 + hundreds(register_x) + tens(register_y) + 0x4
            }
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
        assert_eq!(into_u16(JP(0x234.into())), 0x1234)
    }

    #[test]
    fn as_u16_call() {
        assert_eq!(into_u16(CALL(0x345.into())), 0x2345)
    }

    fn r(n: u8) -> Register {
        Register(n)
    }

    #[test]
    fn as_u16_se_byte() {
        assert_eq!(into_u16(SEByte(r(0x4), 0x56)), 0x3456)
    }

    #[test]
    fn as_u16_sne_byte() {
        assert_eq!(into_u16(SNEByte(r(0x5), 0x67)), 0x4567)
    }

    #[test]
    fn as_u16_se_register() {
        assert_eq!(into_u16(SERegister(r(0xA), r(0xB))), 0x5AB0)
    }

    #[test]
    fn as_u16_sne_register() {
        assert_eq!(into_u16(SNERegister(r(0xA), r(0xB))), 0x9AB0)
    }

    #[test]
    fn as_u16_ld_byte() {
        assert_eq!(into_u16(LDByte(r(0x7), 0x89)), 0x6789);
    }

    #[test]
    fn as_u16_add_byte() {
        assert_eq!(into_u16(ADDByte(r(0x8), 0x9A)), 0x789A)
    }

    #[test]
    fn as_u16_ldi() {
        assert_eq!(into_u16(LDI(0xBCD.into())), 0xABCD)
    }

    #[test]
    fn as_u16_drw() {
        assert_eq!(into_u16(DRW(r(0xA), r(0xB), 0xC)), 0xDABC)
    }

    #[test]
    fn as_u16_addi() {
        assert_eq!(into_u16(ADDI(r(0xB))), 0xFB1E)
    }

    #[test]
    fn as_u16_rnd() {
        assert_eq!(into_u16(RND(r(0xA), 0xBC)), 0xCABC)
    }

    #[test]
    fn as_u16_add_registers() {
        assert_eq!(into_u16(ADDRegister(r(0xA), r(0xB))), 0x8AB4)
    }

    #[test]
    fn from_u16() {
        use std::collections::HashMap;

        #[rustfmt::skip]
        let instructions: HashMap<u16, Instruction> = [
            (0x00EE, RET()),
            (0x0ABC, SYS()),
            (0x1A12, JP(0xA12.into())),
            (0x221A, CALL(0x21A.into())),
            (0x3934, SEByte(r(0x9), 0x34)),
            (0x4A56, SNEByte(r(0xA), 0x56)),
            (0x5730, SERegister(r(0x7), r(0x3))),
            (0x6003, LDByte(r(0x0), 0x03)),
            (0x7123, ADDByte(r(0x1), 0x23)),
            (0x8124, ADDRegister(r(0x1), r(0x2))),
            (0x9AB0, SNERegister(r(0xA), r(0xB))),
            (0xA278, LDI(0x278.into())),
            (0xC123, RND(r(0x1), 0x23)),
            (0xD123, DRW(r(0x1), r(0x2), 0x3)),
            (0xF51E, ADDI(r(0x5)))
        ].iter().cloned().collect();

        for (chunk, instruction) in instructions.into_iter() {
            let actual = Instruction::try_from(chunk).unwrap();
            assert_eq!(actual, instruction);
        }
    }
}
