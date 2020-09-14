# CHIP-8 Emulator

A CHIP-8 emulator in Rust.

## Commands

To print out every command from a CHIP-8 file (like [this
one](https://johnearnest.github.io/chip8Archive/roms/octojam1title.ch8)):

    chip8 print FILE.ch8

To run the program:

    chip8 run FILE.ch8

To run the program with helpful statements indicating what instructions it's
executing:

    chip8 trace FILE.ch8

## Testing

Run tests:

    cargo test

Test that the opcodes work correctly by running the included test ROM:

    cargo run -- trace test_opcode.ch8
