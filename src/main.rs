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
    let print_object_tree = true;
    let print_test_object = true;

    let lexer = AsmLexer::new();
    let code = r#"
    label1:
    loadmd 0 r0
    loadmd 1 r1
    label2:
    loadmd 1 r2
    .section "data"
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
            eprintln!("Error occured while generating object file:\n{}", err);
            return
        }
    }
    if print_object_tree {
        println!("Object tree: {:#?}", objgenerator);
    }

    const TEST_LOCATION: &str = "saved_binary.sao";

    match objgenerator.save_object(TEST_LOCATION) {
        Ok(()) => {},
        Err(e) => {
            eprintln!("Error occured while saving binary into file:\n{}", e);
            return;
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
}
