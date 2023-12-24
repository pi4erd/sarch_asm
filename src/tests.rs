#[cfg(test)]

#[test]
fn recursive_define() {
    use crate::objgen::{ObjectFormat, Constant};

    let code = ".section \"text\"
    .define A 12
    .define B A
    
    start:
    loadid B r0
    halt
    
    .section \"data\"
    .section \"rodata\"
    ";
    let tokens = super::lex(code, false);
    let node = super::parse(tokens, false).unwrap();
    let mut obj = ObjectFormat::new();
    obj.load_parser_node(&node).unwrap();

    let instr = &obj.sections["text"].instructions[0];

    assert_eq!(instr.constants.len(), 2);
    assert_eq!(instr.constants[0], Constant {
        argument_pos: 0,
        size: crate::objgen::ConstantSize::DoubleWord,
        value: 12
    })
}
