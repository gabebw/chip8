use thiserror::Error;

#[derive(Error, Debug)]
pub enum Chip8Error {
    #[error("IO Error: {0:?}")]
    Io(#[from] std::io::Error),
}
