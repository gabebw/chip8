mod test {
    use crate::{error::Chip8Error, instruction::Instruction};
    use std::convert::TryFrom;

    fn parse(program: &[u16]) -> Result<Vec<Instruction>, Chip8Error> {
        program.iter().map(Instruction::try_from).collect()
    }

    #[test]
    fn parse_everything() {
        use crate::instruction::Instruction::*;
        use std::convert::TryInto;

        let input = vec![
            0x00EE, 0x0ABC, 0x1A12, 0x221A, 0x4A56, 0x6003, 0x7123, 0xA278, 0xD123, 0xF51E,
        ];
        let expected = vec![
            RET(),
            SYS(),
            JP(0xA12.try_into().unwrap()),
            CALL(0x21A.try_into().unwrap()),
            SNE(0xA, 0x56),
            LD(0x0, 0x03.try_into().unwrap()),
            ADD(0x1, 0x23),
            LDI(0x278.try_into().unwrap()),
            DRW(0x1, 0x2, 0x3),
            ADDI(0x5),
        ];
        assert_eq!(parse(&input).unwrap(), expected);
    }
}
