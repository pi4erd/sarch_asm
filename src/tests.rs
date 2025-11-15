#[cfg(test)]

#[test]
fn label_defbyte() {
    use std::collections::HashMap;

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

    let mut included = HashMap::new();

    let tokens = super::lex(&mut included, &code, false).unwrap();
    let node = super::parse("test", tokens, false).unwrap();
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
fn sublabel_test() {
    use crate::objgen::ObjectFormat;
    use std::collections::HashMap;

    let code = ".section \"text\"
    
    label1:
    nop
    nop
    @sublabel:
    nop
    halt

    label2:
    nop
    nop
    @sublabel:
    loadid @sublabel, r0
    loadid label1@sublabel, r0
    halt

    .section \"data\"
    .section \"rodata\"
    ";

    let mut included = HashMap::new();
    let tokens = super::lex(&mut included, &code, false).unwrap();
    let node = super::parse("test", tokens, false).unwrap();

    let mut obj = ObjectFormat::new();
    obj.load_parser_node(&node).unwrap();
}

#[test]
fn macro_test() {
    use std::collections::HashMap;
    use crate::lexer::LexerTokenType;
    
    let code = "
    %macro some_macro {
        ; macro content
        loadid 0x00, r0
        ; comment 2
        loadid 0xBAD, ra ; ve
    }

    %macro argumented_macro(hello, world) {
        \\hello
        \\world
    }

    some_macro
    some_macro
    some_macro
    some_macro
    some_macro

    argumented_macro(nop, nop)
    ";

    let mut included = HashMap::new();
    let tokens = super::lex(&mut included, &code, false).unwrap();

    assert!(tokens.iter().find(|t| {
        t.kind == LexerTokenType::PreprocessInstruction ||
        t.kind == LexerTokenType::Comment ||
        t.kind == LexerTokenType::LParen ||
        t.kind == LexerTokenType::RParen
    }).is_none());

    let node = super::parse("test", tokens, false).unwrap();

    println!("{:#?}", node);
}

#[test]
fn recursive_define() {
    use crate::objgen::{ObjectFormat, Constant};

    use std::collections::HashMap;
    
    let code = ".section \"text\"
    .define A 12
    .define B A
    
    start:
    loadid B, r0
    halt
    
    .section \"data\"
    .section \"rodata\"
    ";

    let mut included = HashMap::new();
    let tokens = super::lex(&mut included, &code, false).unwrap();
    let node = super::parse("test", tokens, false).unwrap();
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
    use std::collections::HashMap;
    use crate::objgen::ObjectFormat;
    
    let code = ".section \"text\"
    .define A B
    .define B A
    
    start:
        loadid A, r0
    ; Basically, because B<->A, should detect 100+ loops and then return error
    ; That's why the test is like this

    .section \"data\"
    .section \"rodata\"
    ";


    let mut included = HashMap::new();
    let tokens = super::lex(&mut included, &code, false).unwrap();
    let node = super::parse("test", tokens, false).unwrap();
    let mut obj = ObjectFormat::new();
    let res = obj.load_parser_node(&node);

    if let Err(_) = res {
        return;
    } else {
        panic!("Test failed! Defines should be infinite!");
    }
}

#[test]
fn expression_test() {
    use crate::{objgen::ObjectFormat, parser::NodeType};
    use std::collections::HashMap;

    let code = ".section \"text\"
    .define A 3
    .define B (5 + 2)
    .define C (10 * 5)
    .define D (C * 10)
    
    start:
        loadid C, r0
        loadid B, r1
        loadid (15 * 2900), r2
        loadid (91 + B), r3

        ; this won't work because expressions aren't yet implemented inside object files
        ; loadid (start + 2), r0
        halt
    .section \"data\"
    .section \"rodata\"
    ";
    
    let mut included = HashMap::new();
    let tokens = super::lex(&mut included, &code, false).unwrap();
    let node = super::parse("test", tokens, false).unwrap();
    
    let mut obj = ObjectFormat::new();
    obj.load_parser_node(&node).unwrap();
    
    assert_eq!(obj.defines["A"].node.node_type, NodeType::ConstInteger(3));
    assert_eq!(obj.defines["B"].node.node_type, NodeType::ConstInteger(7));
    assert_eq!(obj.defines["C"].node.node_type, NodeType::ConstInteger(50));
    assert_eq!(obj.defines["D"].node.node_type, NodeType::ConstInteger(500));
}

#[test]
fn include_test() {
    use std::collections::HashMap;
    use crate::lexer::LexerTokenType;

    let code = "
    %include \"tests/test_file.s\"
 
    start: ; comment
        jmp start
    ";

    let mut included = HashMap::new();
    let tokens = super::lex(&mut included, code, false).unwrap();
    
    assert!(tokens.iter().find(|t| t.kind == LexerTokenType::Comment).is_none());

    println!("{tokens:?}");
}

#[test]
fn comma_test() {
    use std::collections::HashMap;

    let code = "
    loadid A, C # correct
    loadid A C # incorrect
    ";

    let mut included = HashMap::new();
    let tokens = super::lex(&mut included, code, false).unwrap();
    let result = super::parse("comma_test", tokens, false);

    assert!(result.is_err(), "No commas between arguments MUST give error.");

    println!("{:?}", result);
}

#[test]
fn lex_test() {
    use crate::lexer::LexerTokenType;
    use std::collections::HashMap;

    let code = ".define ABC 0xFE
    start: ; hello world this is a comment
        loadid C, r0
        loadid (91 + B), r3 ;fgewt
        jmp start # no influence :)
    string: .db \"Hello, world!\"
    ";
    
    let mut included = HashMap::new();
    let tokens = super::lex(&mut included, code, false).unwrap();

    assert_eq!(
        tokens.into_iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![
            LexerTokenType::CompilerInstruction, LexerTokenType::Identifier, LexerTokenType::Integer, LexerTokenType::Newline,

            LexerTokenType::Label, LexerTokenType::Newline,

            LexerTokenType::Identifier, LexerTokenType::Identifier, LexerTokenType::Comma, LexerTokenType::Identifier, LexerTokenType::Newline,

            LexerTokenType::Identifier, LexerTokenType::LParen, LexerTokenType::Integer, LexerTokenType::Plus,
            LexerTokenType::Identifier, LexerTokenType::RParen,  LexerTokenType::Comma, LexerTokenType::Identifier, LexerTokenType::Newline,

            LexerTokenType::Identifier, LexerTokenType::Identifier, LexerTokenType::Newline,
            LexerTokenType::Label, LexerTokenType::CompilerInstruction, LexerTokenType::String, LexerTokenType::Newline,
        ]
    )
}
