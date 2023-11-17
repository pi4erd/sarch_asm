pub mod lexer;
pub mod parser;
pub mod preprocessor;
pub mod symbols;
pub mod objgen;
pub mod linker;

use lexer::AsmLexer;
use parser::Parser;

fn main() {
    let print_tokens = false;
    let print_ast = true;

    let lexer = AsmLexer::new();
    let code = r#"label:
    bin -1
    bin -6.43
    bin
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
