use logos::{Lexer, Logos};
use std::{error::Error, fmt::Display, rc::Rc};

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\f\r]+", extras = (usize, usize))]
enum Token {
    #[regex(r"[\@a-zA-Z_][\@a-zA-Z_0-9]*", character_callback)]
    Identifier((usize, usize)),
    #[regex(r"(?:0x[0-9a-fA-F]+|0b[01]+|(?:0d|)\d+)", character_callback)]
    Integer((usize, usize)),
    #[regex(r"[\@a-zA-Z_][\@a-zA-Z_0-9]*:", character_callback)]
    Label((usize, usize)),
    #[regex(r"(?:\d+\.\d*|\d*\.\d+)", character_callback, priority = 5)]
    FloatingPoint((usize, usize)),
    #[regex(r"\.\w+", character_callback, priority = 4)]
    CompilerInstruction((usize, usize)),
    #[regex(r"\%\w+", character_callback, priority = 4)]
    PreprocessInstruction((usize, usize)),
    #[regex(r#""[^"]*""#, character_callback, priority = 6)]
    String((usize, usize)),
    #[regex(r"'.'", character_callback)]
    Character((usize, usize)),
    #[regex(r"[;#].*", newline_callback)]
    Comment,
    #[token("\n", newline_callback)]
    Newline,
    #[token("\\", character_callback)]
    EscapeChar((usize, usize)),
    #[token("(", character_callback)]
    LParen((usize, usize)),
    #[token(")", character_callback)]
    RParen((usize, usize)),
    #[token("{", character_callback)]
    LBracket((usize, usize)),
    #[token("}", character_callback)]
    RBracket((usize, usize)),
    #[token(",", character_callback)]
    Comma((usize, usize)),
    #[token("+", character_callback)]
    Plus((usize, usize)),
    #[token("-", character_callback)]
    Minus((usize, usize)),
    #[token("*", character_callback)]
    Multiply((usize, usize)),
    #[token("/", character_callback)]
    Divide((usize, usize)),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexerTokenType {
    Identifier,
    Integer,
    Label,
    FloatingPoint,
    CompilerInstruction,
    Newline,
    String,
    Character,
    Comment,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Escaped,
    Comma,
    Plus,
    Minus,
    Multiply,
    Divide,
    PreprocessInstruction,

    EnterInclude,
    ExitInclude,
}

#[derive(Clone, Debug)]
pub struct LexerToken {
    pub kind: LexerTokenType,
    pub slice: Rc<str>,
    pub line: usize,
    pub column: usize,
}

impl LexerToken {
    pub fn expect(&self, token_type: LexerTokenType) -> LexerResult<()> {
        if self.kind != token_type {
            return Err(LexerError::Lexer {
                message: format!(
                    "Unexpected token {:?}. {:?} expected.",
                    self.kind, token_type,
                ),
                line: self.line,
                column: self.column,
            });
        }

        return Ok(());
    }
}

#[derive(Debug)]
pub enum LexerError {
    Lexer {
        message: String,
        line: usize,
        column: usize,
    },
    EOF {
        line: usize,
        column: usize,
    },
    Other {
        error: Box<dyn Error>,
    },
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lexer {
                message,
                line,
                column,
            } => {
                write!(f, "{}: line {} column {}", message, line, column)
            }
            Self::EOF { line, column } => {
                write!(f, "Unexpected EOF: line {} column {}", line, column)
            }
            Self::Other { error } => {
                write!(f, "{error}")
            }
        }
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

fn tokenize_internal<'s>(
    code: &'s str,
    prev_include: Option<&str>,
) -> LexerResult<Vec<LexerToken>> {
    if prev_include.is_some() {
        todo!("including file")
    }

    let mut lex = Token::lexer(code);

    let mut tokens = Vec::new();

    let mut escaping = false;

    while let Some(token) = lex.next() {
        let slice = lex.slice();

        if let Err(_) = token {
            return Err(LexerError::Lexer {
                message: format!("Unrecognized character '{}'.", slice),
                line: lex.extras.0,
                column: lex.extras.1,
            });
        }

        if escaping {
            let token = LexerToken {
                kind: LexerTokenType::Escaped,
                slice: Rc::from(slice),
                line: lex.extras.0,
                column: lex.extras.1,
            };

            tokens.push(token);
            escaping = false;

            continue;
        }

        let token = token.unwrap();

        let token_kind = match &token {
            Token::Identifier(_) => LexerTokenType::Identifier,
            Token::Integer(_) => LexerTokenType::Integer,
            Token::Label(_) => LexerTokenType::Label,
            Token::LParen(_) => LexerTokenType::LParen,
            Token::RParen(_) => LexerTokenType::RParen,
            Token::LBracket(_) => LexerTokenType::LBracket,
            Token::RBracket(_) => LexerTokenType::RBracket,
            Token::Newline => LexerTokenType::Newline,
            Token::Comment => LexerTokenType::Comment,
            Token::EscapeChar(_) => {
                escaping = true;
                continue;
            }
            Token::CompilerInstruction(_) => LexerTokenType::CompilerInstruction,
            Token::String(_) => LexerTokenType::String,
            Token::FloatingPoint(_) => LexerTokenType::FloatingPoint,
            Token::PreprocessInstruction(_) => LexerTokenType::PreprocessInstruction,
            Token::Character(_) => LexerTokenType::Character,
            Token::Comma(_) => LexerTokenType::Comma,
            Token::Plus(_) => LexerTokenType::Plus,
            Token::Minus(_) => LexerTokenType::Minus,
            Token::Multiply(_) => LexerTokenType::Multiply,
            Token::Divide(_) => LexerTokenType::Divide,
        };

        let token = LexerToken {
            kind: token_kind,
            slice: Rc::from(slice),
            line: lex.extras.0,
            column: lex.extras.1,
        };

        tokens.push(token);
    }

    return Ok(tokens);
}

pub fn tokenize<'s>(code: &'s str) -> LexerResult<Vec<LexerToken>> {
    let tokens = tokenize_internal(code, None)?;

    Ok(tokens)
}
