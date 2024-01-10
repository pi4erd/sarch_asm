#[cfg(test)]

#[test]
fn label_defbyte() {
    use crate::{objgen::ObjectFormat, linker::Linker};

    let code = ".section \"text\"
    
    label1:
    nop
    nop
    label2:
    nop
    nop
    label3:
    nop
    nop
    
    .section \"data\"
    .db label1
    .dw label2
    .dd label3

    .section \"rodata\"
    ";

    let tokens = super::lex(code, false);
    let node = super::parse(tokens, false).unwrap();
    let mut obj = ObjectFormat::new();
    obj.load_parser_node(&node).unwrap();

    let mut linker = Linker::new();
    linker.load_symbols(obj).unwrap();

    let binary = linker.generate_binary(None).unwrap();
    let mut bin_check: Vec<u8> = vec![
        0, 0, 0, 0, 0, 0, // nops
    ];
    while bin_check.len() < 256 {
        bin_check.push(0);
    }
    bin_check.append(&mut vec![0, 2, 0, 4, 0, 0, 0]);
    while bin_check.len() < 512 {
        bin_check.push(0);
    }

    assert_eq!(bin_check.len(), 512);
    assert_eq!(binary.len(), 512);
    assert_eq!(binary, bin_check);
}

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
    assert_eq!(instr.references.len(), 0);
    assert_eq!(instr.constants[0], Constant {
        argument_pos: 0,
        size: crate::objgen::ConstantSize::DoubleWord,
        value: 12
    })
}

#[test]
fn infinite_define() {
    use crate::objgen::ObjectFormat;
    let code = ".section \"text\"
    .define A B
    .define B A
    
    start:
        loadid A r0
    ; Basically, because B<->A, should detect 100+ loops and then return error
    ; That's why the test is like this

    .section \"data\"
    .section \"rodata\"
    ";
    let tokens = super::lex(code, false);
    let node = super::parse(tokens, false).unwrap();
    let mut obj = ObjectFormat::new();
    let res = obj.load_parser_node(&node);

    if let Err(_) = res {
        return;
    } else {
        panic!("Test failed! Defines should be infinite!");
    }
}