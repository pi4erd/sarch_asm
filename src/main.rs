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
    let print_ast = true;

    let lexer = AsmLexer::new();
    let code = r#"label:
    @sublabel:
    .section "text"
    .section "data"
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

    const TEST: bool = true;

    if TEST {
        // 0x3A6863FC6173371B
        let object_bin: Vec<u8> = vec![
            // header
            0x1B, 0x37, 0x73, 0x61, 0xFC, 0x63, 0x68, 0x3A, // magic
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // labelinfo
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // instructions
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // data
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sections
            0x01, 0x00, 0x00, 0x00, // version

            // labels
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // ptr
            0x00, // ptr data
            b'H', b'e', b'l', b'l', 0x00, // name

            // instructions
            0x00, 0x00, // opcode
            0x01, // reference count
            0x00, // constant count
            // reference
            0x00, // argument position
            b'H', b'e', b'l', b'l', 0x00, // reference name
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
    println!("{:?}", objgenerator);
}
