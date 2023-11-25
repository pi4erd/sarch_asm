pub mod lexer;
pub mod parser;
pub mod preprocessor;
pub mod symbols;
pub mod objgen;
pub mod linker;

use lexer::AsmLexer;
use parser::Parser;

use crate::objgen::ObjectFormat;

fn main() {
    let print_tokens = false;
    let print_ast = false;

    let lexer = AsmLexer::new();
    let code = r#"
    .define test 0.0
    label1:
    loadmd test r3
    loadmd test r3
    @bruh:
    loadmd test r3
    loadmd test r3
    @dir:
    loadmd test r3
    .section "data"
    label2:
"#;
    let tokens = lexer.tokenize(code);

    if print_tokens {
        for token in tokens.iter() {
            println!("{:?}", token);
        }
    }

    let mut parser = Parser::new();
    let node = match parser.parse(&tokens) {
        Ok(n) => n,
        Err(err) => {
            eprintln!("Error occured while parsing:\n{}", err);
            return;
        }
    };

    if print_ast {
        println!("{:#?}", node);
    }

    const TEST: bool = false;

    if TEST {
        // 0x3A6863FC6173371B
        let object_bin: Vec<u8> = vec![
            // header
            0x1B, 0x37, 0x73, 0x61, 0xFC, 0x63, 0x68, 0x3A, // magic
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // section count
            0x03, 0x00, 0x00, 0x00, // version

            // section
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // instruction count
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // label count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // binary size
            b't', b'e', b'x', b't', 0x00, // section name
            // label
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // ptr
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // ptr bin
            b'H', b'e', b'l', b'l', 0x00, // label name
            // instruction
            0x00, 0x00, // opcode
            0x01, // ref count
            0x01, // const count
                // ref
            0x01, // arg pos
            b'H', b'e', b'l', b'l', 0x00, // ref name
                // const
            0x00, // arg pos
            0x02, // size (in bytes)
            0x40, 0x10, // const (0x1040)
            // binary

            // section
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // instruction count
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // label count
            0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // binary size
            b'd', b'a', b't', b'a', 0x00, // section name
            // label
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // ptr
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // ptr bin
            b'b', b'e', b'e', b'f', 0x00, // label name
            // binary
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // binmsg

            0x00
        ];

        let obj = ObjectFormat::from_bytes(object_bin).unwrap();

        println!("{:#?}", obj);
    }

    let mut objgenerator = objgen::ObjectFormat::new();
    match objgenerator.load_parser_node(node) {
        Ok(()) => {},
        Err(err) => {
            eprintln!("Error occured while generating object file:\n{}", err);
            return
        }
    }
    println!("{:#?}", objgenerator);
}
