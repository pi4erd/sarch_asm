use regex_lexer::Token;
use crate::lexer::LexerToken;
use std::collections::HashMap;

macro_rules! returnerr {
    ($token:expr) => {
        return Err(format!("Unexpected token {:?} \"{}\" at {}..{}", 
            $token.kind, $token.text, $token.span.start, $token.span.end))
    };
}

macro_rules! unwrap_from_option {
    ($option:expr) => {
        match $option {
            Some(n) => n,
            None => {
                return Err(format!("Unexpected EOF at the end!"))
            }
        }
    }
}

// TODO: Split registers into 32, 16 and 8 bit registers for the better life
pub struct Registers<'a> {
    registers32: HashMap<&'a str, u8>,
    registers8: HashMap<&'a str, u8>,
    registers16: HashMap<&'a str, u8>
}

impl Registers<'_> {
    pub fn new<'a>() -> Self {
        let mut me = Self {
            registers32: HashMap::new(),
            registers8: HashMap::new(),
            registers16: HashMap::new(),
        };

        // 32 bit
        me.registers32.insert("r0", 0);
        me.registers32.insert("r1", 1);
        me.registers32.insert("r2", 2);
        me.registers32.insert("r3", 3);
        me.registers32.insert("r4", 4);
        me.registers32.insert("r5", 5);
        me.registers32.insert("r6", 6);
        me.registers32.insert("r7", 7);
        me.registers32.insert("r8", 8);
        me.registers32.insert("r9", 9);
        me.registers32.insert("ra", 10);
        me.registers32.insert("rb", 11);
        me.registers32.insert("rc", 12);
        me.registers32.insert("rd", 13);
        me.registers32.insert("re", 14);
        me.registers32.insert("rf", 15);
        me.registers32.insert("ip", 16);
        me.registers32.insert("sr", 17);
        me.registers32.insert("mfr", 18);
        me.registers32.insert("sp", 19);
        me.registers32.insert("bp", 20);
        me.registers32.insert("tptr", 21);

        // 16 bit
        me.registers16.insert("r00", 0);
        me.registers16.insert("r01", 1);
        me.registers16.insert("r10", 2);
        me.registers16.insert("r11", 3);
        me.registers16.insert("r20", 4);
        me.registers16.insert("r21", 5);
        me.registers16.insert("r30", 6);
        me.registers16.insert("r31", 7);
        me.registers16.insert("r40", 8);
        me.registers16.insert("r41", 9);
        me.registers16.insert("r50", 10);
        me.registers16.insert("r51", 11);
        me.registers16.insert("r60", 12);
        me.registers16.insert("r61", 13);
        me.registers16.insert("r70", 14);
        me.registers16.insert("r71", 15);
        me.registers16.insert("r80", 16);
        me.registers16.insert("r81", 17);
        me.registers16.insert("r90", 18);
        me.registers16.insert("r91", 19);
        me.registers16.insert("ra0", 20);
        me.registers16.insert("ra1", 21);
        me.registers16.insert("rb0", 22);
        me.registers16.insert("rb1", 23);
        me.registers16.insert("rc0", 24);
        me.registers16.insert("rc1", 25);
        me.registers16.insert("rd0", 26);
        me.registers16.insert("rd1", 27);
        me.registers16.insert("re0", 28);
        me.registers16.insert("re1", 29);
        me.registers16.insert("rf0", 30);
        me.registers16.insert("rf1", 31);

        // 8 bit
        me.registers8.insert("r00l", 0);
        me.registers8.insert("r00h", 1);
        me.registers8.insert("r01l", 2);
        me.registers8.insert("r01h", 3);
        me.registers8.insert("r10l", 4);
        me.registers8.insert("r10h", 5);
        me.registers8.insert("r11l", 6);
        me.registers8.insert("r11h", 7);
        me.registers8.insert("r20l", 8);
        me.registers8.insert("r20h", 9);
        me.registers8.insert("r21l", 10);
        me.registers8.insert("r21h", 11);
        me.registers8.insert("r30l", 12);
        me.registers8.insert("r30h", 13);
        me.registers8.insert("r31l", 14);
        me.registers8.insert("r31h", 15);
        me.registers8.insert("r40l", 16);
        me.registers8.insert("r40h", 17);
        me.registers8.insert("r41l", 18);
        me.registers8.insert("r41h", 19);
        me.registers8.insert("r50l", 20);
        me.registers8.insert("r50h", 21);
        me.registers8.insert("r51l", 22);
        me.registers8.insert("r51h", 23);
        me.registers8.insert("r60l", 24);
        me.registers8.insert("r60h", 25);
        me.registers8.insert("r61l", 26);
        me.registers8.insert("r61h", 27);
        me.registers8.insert("r70l", 28);
        me.registers8.insert("r70h", 29);
        me.registers8.insert("r71l", 30);
        me.registers8.insert("r71h", 31);

        me
    }

    pub fn get32<'a>(&'a self, key: &'a str) -> Option<&u8> {
        self.registers32.get(key)
    }

    pub fn get16<'a>(&'a self, key: &'a str) -> Option<&u8> {
        self.registers16.get(key)
    }

    pub fn get8<'a>(&'a self, key: &'a str) -> Option<&u8> {
        self.registers8.get(key)
    }

    pub fn get_name8<'a>(&'a self, idx: u8) -> Option<&str> {
        match self.registers8.iter().find(|(_, r)| **r == idx) {
            Some((rn, _)) => Some(rn),
            None => None
        }
    }

    pub fn get_name32<'a>(&'a self, idx: u8) -> Option<&str> {
        match self.registers32.iter().find(|(_, r)| **r == idx) {
            Some((rn, _)) => Some(rn),
            None => None
        }
    }

    pub fn get_name16<'a>(&'a self, idx: u8) -> Option<&str> {
        match self.registers16.iter().find(|(_, r)| **r == idx) {
            Some((rn, _)) => Some(rn),
            None => None
        }
    }

    pub fn has_key<'a>(&'a self, key: &'a str) -> bool {
        self.registers32.contains_key(key) || self.registers16.contains_key(key)
            || self.registers8.contains_key(key)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    ConstInteger(i64),
    ConstFloat(f64),
    Negate,
    Instruction(String),
    CompilerInstruction(String),
    Label(String),
    Identifier(String),
    Register(String),
    String(String),
    Expression,
    Addition,
    Subtraction,
    Multiplication,
    Division,
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
    pub root: ParserNode,
    last_label: String
}

impl Parser {
    pub fn new() -> Self {
        Self { root: ParserNode::new(), last_label: "".to_string() }
    }

    pub fn parse(&mut self, tokens: &Vec<Token<LexerToken>>) -> Result<&ParserNode, String> {
        let mut iterator = tokens.iter();
        while let Some(token) = iterator.next() {
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
                    let txt: &str = &token.text[..token.text.len() - 1];

                    let label_text: String;

                    if txt.starts_with('@') {
                        label_text = self.last_label.clone() + txt;
                    } else {
                        label_text = txt.to_string();
                        self.last_label = label_text.clone();
                    }

                    let node = ParserNode {
                        node_type: NodeType::Label(label_text),
                        children: Vec::new()
                    };

                    self.root.children.push(node);
                }
                LexerToken::Newline => {}
                LexerToken::Comment => {}
                _ => returnerr!(token)
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

        let mut token = match tokens.next() {
            Some(tok) => tok,
            None => return Ok(node)
        };

        let mut argc = 0;

        while token.kind != LexerToken::Newline && token.kind != LexerToken::Comment && argc < 2 {
            let nd = Parser::parse_expression(token, tokens, true, false)?;

            node.children.push(nd);

            token = unwrap_from_option!(tokens.next());
            argc += 1;
        }

        Ok(node)
    }

    fn parse_compiler_instruction<'a>(current_token: &Token<'a, LexerToken>,
        tokens: &mut core::slice::Iter<'a, Token<'a, LexerToken>>)
        -> Result<ParserNode, String>
    {
        let mut node = ParserNode {
            node_type: NodeType::CompilerInstruction(
                current_token.text[1..current_token.text.len()].to_string()
            ),
            children: Vec::new()
        };

        let mut token = unwrap_from_option!(tokens.next());

        while token.kind != LexerToken::Newline && token.kind != LexerToken::Comment {
            let nd = Parser::parse_expression(token, tokens, false, true)?;

            node.children.push(nd);

            token = unwrap_from_option!(tokens.next());
        }

        Ok(node)
    }

    fn parse_expression<'a>(current_token: &Token<'a, LexerToken>,
        tokens: &mut core::slice::Iter<'a, Token<'a, LexerToken>>,
        use_registers: bool, str_available: bool
    )
        -> Result<ParserNode, String>
    {
        let rgs = Registers::new();
        match current_token.kind {
            LexerToken::Integer => {
                let mut numtxt = current_token.text;
                let try_convert: Result<i64, std::num::ParseIntError>;

                if numtxt.starts_with("0x") {
                    numtxt = numtxt.strip_prefix("0x").unwrap();
                    try_convert = i64::from_str_radix(numtxt, 16);
                } else if numtxt.starts_with("0b") {
                    numtxt = numtxt.strip_prefix("0b").unwrap();
                    try_convert = i64::from_str_radix(numtxt, 2);
                } else if numtxt.starts_with("0d") {
                    numtxt = numtxt.strip_prefix("0d").unwrap();
                    try_convert = i64::from_str_radix(numtxt, 10);
                } else {
                    try_convert = i64::from_str_radix(numtxt, 10);
                }

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
            LexerToken::Char => {
                let char = match current_token.text[1..current_token.text.chars().count() - 1].bytes().next() {
                    Some(c) => c,
                    None => {
                        return Err(format!("Cannot parse nonexistant character in Char!"))
                    }
                };
                let node = ParserNode {
                    node_type: NodeType::ConstInteger(char as i64),
                    children: Vec::new()
                };
                Ok(node)
            }
            // TODO: Add chaining expressions without adding more parenthesis
            LexerToken::LParen => { // Used for creating expressions
                let mut next = unwrap_from_option!(tokens.next());

                let lhs = Parser::parse_expression(next, tokens, use_registers, str_available)?;
                next = unwrap_from_option!(tokens.next());
                let operator = next.clone();
                next = unwrap_from_option!(tokens.next());
                let rhs = Parser::parse_expression(next, tokens, use_registers, str_available)?;

                let node = ParserNode {
                    node_type: match operator.kind {
                        LexerToken::Plus => NodeType::Addition,
                        LexerToken::Minus => NodeType::Subtraction,
                        LexerToken::Multiply => NodeType::Multiplication,
                        LexerToken::Divide => NodeType::Division,
                        _ => returnerr!(operator)
                    },
                    children: vec![lhs, rhs]
                };
                let result = ParserNode {
                    node_type: NodeType::Expression,
                    children: vec![node]
                };

                next = unwrap_from_option!(tokens.next());

                if next.kind != LexerToken::RParen {
                    returnerr!(next)
                }
                Ok(result)
            }
            LexerToken::String => {
                if !str_available {
                    return Err(format!("Using String where not allowed: {} at {}..{}",
                    current_token.text, current_token.span.start, current_token.span.end))
                }
                let _str = &current_token.text[1..current_token.text.chars().count() - 1];
                let node = ParserNode {
                    node_type: NodeType::String(_str.to_string()),
                    children: Vec::new()
                };
                Ok(node)
            }
            LexerToken::FloatingPoint => {
                let numtxt = current_token.text;
                let try_convert = numtxt.parse::<f64>();
                let num = match try_convert {
                    Ok(n) => n,
                    Err(err) => {
                        return Err(format!("Error occured while parsing an expression:\n{}", err))
                    }
                };
                let node = ParserNode {
                    node_type: NodeType::ConstFloat(num),
                    children: Vec::new()
                };
                Ok(node)
            }
            LexerToken::Minus => {
                let next = unwrap_from_option!(tokens.next());
                let p_node = Parser::parse_expression(next, tokens, use_registers, str_available)?;
                let node = ParserNode {
                    node_type: NodeType::Negate,
                    children: vec![p_node]
                };
                Ok(node)
            }
            LexerToken::Plus => {
                let next = unwrap_from_option!(tokens.next());
                let node = Parser::parse_expression(next, tokens, use_registers, str_available)?;
                Ok(node)
            }
            LexerToken::Identifier => {
                if rgs.has_key(current_token.text) {
                    if !use_registers {
                        return Err(
                            format!("Register identifier used in incorrect context in \"{}\" at {}..{}",
                                current_token.text, current_token.span.start, current_token.span.end
                            )
                        )
                    }
                    let node = ParserNode {
                        node_type: NodeType::Register(current_token.text.to_string()),
                        children: Vec::new()
                    };
                    return Ok(node)
                }
                let node = ParserNode {
                    node_type: NodeType::Identifier(current_token.text.to_string()),
                    children: Vec::new()
                };
                Ok(node)
            }
            _ => returnerr!(current_token)
        }
    }
}
