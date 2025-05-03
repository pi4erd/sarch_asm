use std::{error::Error, fmt::Display};
use logos::{Lexer, Logos};

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\f\r]+", extras = (usize, usize))]
enum Token {
    #[regex(r"[\@a-zA-Z_][\@a-zA-Z_0-9]*", character_callback)] Identifier((usize, usize)),
    #[regex(r"(?:0x[0-9a-fA-F]+|0b[01]+|(?:0d|)\d+)", character_callback)] Integer((usize, usize)),
    #[regex(r"[\@a-zA-Z_][\@a-zA-Z_0-9]*:", character_callback)] Label((usize, usize)),
    #[regex(r"(?:\d+\.\d*|\d*\.\d+)", character_callback, priority = 5)] FloatingPoint((usize, usize)),
    #[regex(r"\.\w+", character_callback, priority = 4)] CompilerInstruction((usize, usize)),
    #[token("\n", newline_callback)] Newline,
    #[regex(r#""[^"]*""#, character_callback)] String((usize, usize)),
    #[regex(r"'.'", character_callback)] Character((usize, usize)),
    #[regex(r"[;#].*\n", newline_callback)] Comment,
    #[token("(", character_callback)] LParen((usize, usize)),
    #[token(")", character_callback)] RParen((usize, usize)),
    #[token(",", character_callback)] Comma((usize, usize)),
    #[token("+", character_callback)] Plus((usize, usize)),
    #[token("-", character_callback)] Minus((usize, usize)),
    #[token("*", character_callback)] Multiply((usize, usize)),
    #[token("/", character_callback)] Divide((usize, usize)),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexerTokenType {
    Identifier, Integer, Label, FloatingPoint,
    CompilerInstruction, Newline, String,
    Character, Comment, LParen, RParen, Comma, Plus,
    Minus, Multiply, Divide
}

#[derive(Clone, Copy, Debug)]
pub struct LexerToken<'s> {
    pub kind: LexerTokenType,
    pub slice: &'s str,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct LexerError {
    message: String,
    line: usize,
    column: usize,
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: line {} column {}", self.message, self.line, self.column)
    }
}
impl Error for LexerError {}

pub type LexerResult<T> = Result<T, LexerError>;

fn newline_callback(lex: &mut Lexer<Token>) {
    lex.extras.0 += 1;
    lex.extras.1 = lex.span().end;
}

fn character_callback(lex: &mut Lexer<Token>) -> (usize, usize) {
    let line = lex.extras.0;
    let column = lex.span().start - lex.extras.1;

    (line + 1, column + 1)
}

pub fn tokenize<'s>(code: &'s str) -> LexerResult<Vec<LexerToken<'s>>> {
    let mut lex = Token::lexer(code);

    let mut tokens = Vec::new();

    while let Some(token) = lex.next() {
        let slice = lex.slice();

        if let Err(_) = token {
            return Err(LexerError {
                message: format!("Unrecognized character '{}'.", slice),
                line: lex.extras.0,
                column: lex.extras.1,
            })
        }

        let token = token.unwrap();

        let token = match token {
            Token::Identifier((line, column)) => LexerToken {
                kind: LexerTokenType::Identifier,
                slice,
                line,
                column,
            },
            Token::Integer((line, column)) => LexerToken {
                kind: LexerTokenType::Integer,
                slice,
                line,
                column,
            },
            Token::Label((line, column)) => LexerToken {
                kind: LexerTokenType::Label,
                slice,
                line,
                column,
            },
            Token::FloatingPoint((line, column)) => LexerToken {
                kind: LexerTokenType::FloatingPoint,
                slice,
                line,
                column,
            },
            Token::CompilerInstruction((line, column)) => LexerToken {
                kind: LexerTokenType::CompilerInstruction,
                slice,
                line,
                column,
            },
            Token::Comment |
            Token::Newline => LexerToken {
                kind: LexerTokenType::Newline,
                slice,
                line: lex.extras.0,
                column: lex.extras.1,
            },
            Token::String((line, column)) => LexerToken {
                kind: LexerTokenType::String,
                slice,
                line,
                column,
            },
            Token::Character((line, column)) => LexerToken {
                kind: LexerTokenType::Character,
                slice,
                line,
                column,
            },
            Token::LParen((line, column)) => LexerToken {
                kind: LexerTokenType::LParen,
                slice,
                line,
                column,
            },
            Token::RParen((line, column)) => LexerToken {
                kind: LexerTokenType::RParen,
                slice,
                line,
                column,
            },
            Token::Comma((line, column)) => LexerToken {
                kind: LexerTokenType::Comma,
                slice,
                line,
                column,
            },
            Token::Plus((line, column)) => LexerToken {
                kind: LexerTokenType::Plus,
                slice,
                line,
                column,
            },
            Token::Minus((line, column)) => LexerToken {
                kind: LexerTokenType::Minus,
                slice,
                line,
                column,
            },
            Token::Multiply((line, column)) => LexerToken {
                kind: LexerTokenType::Multiply,
                slice,
                line,
                column,
            },
            Token::Divide((line, column)) => LexerToken {
                kind: LexerTokenType::Divide,
                slice,
                line,
                column,
            },
        };

        tokens.push(token);
    }

    Ok(tokens)
}
