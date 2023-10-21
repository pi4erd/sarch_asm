use regex_lexer::{Token};
use crate::lexer::LexerToken;

macro_rules! returnerr {
    ($tokenkind:expr, $tokenspan:expr) => {
        return Err(format!("Unexpected token {:?} at {}..{}", 
            $tokenkind, $tokenspan.start, $tokenspan.end))
    };
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    ConstInteger(i64),
    ConstFloat(f64),
    Expression,
    Instruction(String),
    CompilerInstruction(String),
    Label(String),
    Identifier(String),
    Register(String),
    String(String),
    Program
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParserNode {
    pub node_type: NodeType,
    pub children: Vec<ParserNode>
}

impl ParserNode {
    pub fn new() -> Self {
        Self { children: Vec::new(), node_type: NodeType::Program }
    }
}

pub struct Parser {
    pub root: ParserNode
}

impl Parser {
    pub fn new() -> Self {
        Self { root: ParserNode::new() }
    }

    pub fn parse(&mut self, tokens: &Vec<Token<LexerToken>>) -> Result<&ParserNode, String> {
        let mut iterator = tokens.iter();
        while let Some(mut token) = iterator.next() {
            match token.kind { // Highest level match
                LexerToken::CompilerInstruction => {
                    let instruction = Parser::parse_compiler_instruction(token, &mut iterator)?;
                    self.root.children.push(instruction);
                }
                LexerToken::Identifier => {
                    let instruction = Parser::parse_instruction(token, &mut iterator)?;
                    self.root.children.push(instruction);
                }
                LexerToken::Label => {
                    let label_text = &token.text[..token.text.len() - 1];

                    let node = ParserNode {
                        node_type: NodeType::Label(label_text.to_string()),
                        children: Vec::new()
                    };

                    self.root.children.push(node);
                }
                LexerToken::Newline => {}
                LexerToken::Comment => {}
                _ => returnerr!(token.kind, token.span)
            }
        }

        Ok(&self.root)
    }

    fn parse_instruction<'a>(current_token: &Token<'a, LexerToken>,
        tokens: &mut core::slice::Iter<'a, Token<'a, LexerToken>>)
        -> Result<ParserNode, String>
    {
        let mut node = ParserNode {
            node_type: NodeType::Instruction(current_token.text.to_string()),
            children: Vec::new()
        };

        let mut peekable = tokens.peekable();

        let next = peekable.peek().unwrap(); // FIXME: Unsafe with unwrap

        if next.kind == LexerToken::Newline {
            return Ok(node);
        }

        let mut token = tokens.next().unwrap();

        let mut argc = 0;

        while token.kind != LexerToken::Newline {
            match token.kind {
                LexerToken::Identifier => {
                    let iden = ParserNode {
                        node_type: NodeType::Identifier(token.text.to_string()),
                        children: Vec::new()
                    };
                    node.children.push(iden);
                }
                LexerToken::Integer => {
                    let cint: ParserNode = 
                        Parser::parse_expression(token, tokens)?;
                    node.children.push(cint);
                }
                _ => returnerr!(token.kind, token.span)
            }
            token = tokens.next().unwrap();
            argc += 1;
        }

        Ok(node)
    }

    fn parse_compiler_instruction<'a>(current_token: &Token<'a, LexerToken>,
        tokens: &mut core::slice::Iter<'a, Token<'a, LexerToken>>)
        -> Result<ParserNode, String>
    {
        todo!()
    }

    fn parse_expression<'a>(current_token: &Token<'a, LexerToken>,
        tokens: &mut core::slice::Iter<'a, Token<'a, LexerToken>>)
        -> Result<ParserNode, String>
    {
        match current_token.kind {
            LexerToken::Integer => {
                let numtxt = current_token.text;
                let try_convert = numtxt.parse::<i64>();
                let num = match try_convert {
                    Ok(n) => n,
                    Err(err) => {
                        return Err(format!("Error occured while parsing an expression:\n{}", err))
                    }
                };
                let node = ParserNode {
                    node_type: NodeType::ConstInteger(num),
                    children: Vec::new()
                };
                Ok(node)
            }
            LexerToken::LParen => {
                todo!()
            }
            LexerToken::FloatingPoint => {
                todo!()
            }
            LexerToken::Minus => {
                todo!()
            }
            LexerToken::Plus => {
                todo!()
            }
            LexerToken::Identifier => {
                todo!()
            }
            _ => returnerr!(current_token.kind, current_token.span)
        }
    }
}
