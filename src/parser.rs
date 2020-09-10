use crate::{error::Chip8Error, instruction::Instruction};
use std::convert::TryFrom;

pub fn parse(program: &[u16]) -> Result<Vec<Instruction>, Chip8Error> {
    program.iter().map(Instruction::try_from).collect()
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn parse_everything() {
        use crate::instruction::Instruction::*;
        use std::convert::TryInto;

        let input = vec![0x00EE, 0x0ABC, 0x1A12, 0x221A, 0x6003, 0xD123, 0xA278];
        let expected = vec![
            RET(),
            SYS(),
            JP(0xA12.try_into().unwrap()),
            CALL(0x21A.try_into().unwrap()),
            LD(0x0, 0x03.try_into().unwrap()),
            DRW(0x1, 0x2, 0x3),
            LDI(0x278.try_into().unwrap()),
        ];
        assert_eq!(parse(&input).unwrap(), expected);
    }
}
