use thiserror::Error;

#[derive(Error, Debug)]
pub enum Chip8Error {
    #[error("Nibble too large (got {0:4X}, expected <= 0x0FFF)")]
    NibbleTooLarge(u16),
    #[error("IO Error: {0:?}")]
    Io(#[from] std::io::Error),
}
