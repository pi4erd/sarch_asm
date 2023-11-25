use regex_lexer::{LexerBuilder, Lexer, Token};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LexerToken {
    Label, Identifier, Integer, Newline, String, Char, CompilerInstruction,
    Comment, LParen, RParen, Comma, Plus, Minus, FloatingPoint, Multiply, Divide
}

pub struct AsmLexer {
    lex_internal: Lexer<LexerToken>
}

impl AsmLexer {
    // TODO: Add octal support!
    fn build_lexer() -> Lexer<LexerToken> {
        let result = LexerBuilder::new()
            .token(r"[A-Za-z0-9_]+", LexerToken::Identifier)
            .token(r"^(?:\@|)[A-Za-z0-9_]+:", LexerToken::Label)
            .token(r"(?:(0x)[0-9a-fA-F]+|(0b)[01]+|(0d|)\d+)", LexerToken::Integer)
            .token(r"\d+\.\d*", LexerToken::FloatingPoint)
            .token(r"\n", LexerToken::Newline)
            .token(r#"".*""#, LexerToken::String)
            .token(r"^\.\w+", LexerToken::CompilerInstruction)
            .token(r"'.'", LexerToken::Char)
            .token(r"[;#].*\n", LexerToken::Comment)
            .token(r"\(", LexerToken::LParen)
            .token(r"\)", LexerToken::RParen)
            .token(r",", LexerToken::Comma)
            .token(r"\+", LexerToken::Plus)
            .token(r"-", LexerToken::Minus)
            .token(r"\*", LexerToken::Multiply)
            .token(r"\/", LexerToken::Divide)
            .ignore(r"[\t\r ]")
            .build().unwrap();
        result
    }
    pub fn new() -> Self {
        Self { lex_internal: AsmLexer::build_lexer() }
    }
    pub fn tokenize<'a>(self, query: &'a str) -> Vec<Token<'a, LexerToken>> {
        let tokens = self.lex_internal.tokens(query);

        let mut result = Vec::<Token<LexerToken>>::new();

        for token in tokens {
            result.push(token);
        }

        result
    }
}