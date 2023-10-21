pub mod lexer;
pub mod parser;

use lexer::AsmLexer;
use parser::Parser;

fn main() {
    let lexer = AsmLexer::new();
    let _str = r#"thereweboom: hello a, 6
"#;
    let tokens = lexer.tokenize(_str);

    for token in tokens.iter() {
        println!("{:?}", token);
    }

    let mut parser = Parser::new();
    let node = parser.parse(&tokens).unwrap();

    println!("{:?}", node);
}
