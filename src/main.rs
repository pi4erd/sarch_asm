pub mod lexer;
pub mod parser;
pub mod preprocessor;
pub mod symbols;
pub mod objgen;
pub mod linker;

use lexer::AsmLexer;
use parser::Parser;

use crate::{objgen::ObjectFormat, linker::Linker};

#[test]
fn alignment_test() {
    const ALIGNMENT: u64 = 0x100;

    let offset = 0x102u64;

    let tmp = (offset / ALIGNMENT) * ALIGNMENT;
    let result = if offset > tmp { tmp + ALIGNMENT } else { tmp };

    assert_eq!(result, 0x200)
    // THAT SURPRISINGLY WORKS!
}

fn main() {
    let print_tokens = false;
    let print_ast = false;
    let print_object_tree = false;
    let print_test_object = false;
    let generate_binary = false;

    let lexer = AsmLexer::new();
    let code = r#"
    .section "text"
    loadmd message sp
    halt

    .section "data"

    message:
    .db 0x10 0x11 0x12 0x13

    .section "rodata"
"#;
    let tokens = lexer.tokenize(code);

    if print_tokens {
        for token in tokens.iter() {
            println!("Tokens: {:?}", token);
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
        println!("Parser tree: {:#?}", node);
    }

    let mut objgenerator = ObjectFormat::new();
    match objgenerator.load_parser_node(node) {
        Ok(()) => {},
        Err(err) => {
            eprintln!("Error occured while generating object file:\n{}", err)
        }
    }
    if print_object_tree {
        println!("Object tree: {:#?}", objgenerator);
    }

    const TEST_LOCATION: &str = "saved_binary.sao";

    match objgenerator.save_object(TEST_LOCATION) {
        Ok(()) => {},
        Err(e) => {
            eprintln!("Error occured while saving binary into file:\n{}", e)
        }
    }

    let test_obj = match ObjectFormat::from_file(TEST_LOCATION) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error occured while loading object from file:\n{}", e);
            return;
        }
    };

    if print_test_object {
        println!("Test object tree: {:#?}", test_obj);
    }

    let mut linker = Linker::new();
    
    match linker.load_symbols(test_obj) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Error occured while loading a symbol in linker: {e}")
        }
    };

    if generate_binary {
        let binary = match linker.generate_binary(None) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Linker error occured while generating binary: {e}");
                return;
            }
        };
    
        println!("Length: {}\n{:?}", binary.len(), binary);
    }
    
    match linker.save_binary("testbin", None) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Error occured while linking: {e}");
            return
        }
    };
}
