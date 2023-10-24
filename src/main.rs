pub mod lexer;
pub mod parser;

use lexer::AsmLexer;
use parser::Parser;

fn main() {
    let print_tokens = false;
    let print_ast = false;

    let lexer = AsmLexer::new();
    let code = r#".db "Hello, world!"
    let "Yeah no"
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
}
