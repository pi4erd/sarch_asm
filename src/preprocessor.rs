use std::collections::HashMap;

use crate::lexer::{LexerError, LexerResult, LexerToken, LexerTokenType};

pub struct Preprocessor<'a> {
    included: &'a mut HashMap<String, String>,
}

impl<'a> Preprocessor<'a> {
    pub fn new(
        included: &'a mut HashMap<String, String>,
    ) -> Self {
        Self {
            included,
        }
    }

    pub fn preprocess(
        &mut self,
        tokens: Vec<LexerToken>
    ) -> LexerResult<Vec<LexerToken>> {
        let mut new = Vec::new();

        let mut token_iter = tokens.into_iter().peekable();

        while let Some(token) = token_iter.next() {
            match token.kind {
                LexerTokenType::PreprocessInstruction => {
                    let instruction_name = &token.slice[1..token.slice.len()]; 

                    self.run_instruction(
                        instruction_name,
                        &mut new,
                        token.clone(),
                        &mut token_iter
                    )?;
                }
                LexerTokenType::Comment => {}
                _ => new.push(token),
            }
        }

        Ok(new)
    }

    fn run_instruction<I>(
        &mut self,
        instruction_name: &str,
        new_tokens: &mut Vec<LexerToken>,
        prev_token: LexerToken,
        token_iter: &mut I,
    ) -> LexerResult<()>
    where
        I: Iterator<Item = LexerToken>,
    {
        match instruction_name {
            "include" => {
                instructions::include(self.included, new_tokens, prev_token, token_iter)
            },
            _ => return Err(LexerError::Lexer {
                message: format!("unknown preprocessor instruction: {}", instruction_name),
                line: prev_token.line,
                column: prev_token.column,
            })
        }
    }
}

mod instructions {
    use std::{collections::HashMap, fs, io::Read, rc::Rc};

    use crate::{lexer::{LexerError, LexerResult, LexerToken, LexerTokenType, tokenize}, preprocessor::Preprocessor};
    
    pub fn include<'a, I>(
        included: &mut HashMap<String, String>,
        new_tokens: &mut Vec<LexerToken>,
        token: LexerToken,
        token_iter: &mut I,
    ) -> LexerResult<()> where
        I: Iterator<Item = LexerToken>
    {
        let new_token = token_iter.next();

        if new_token.is_none() {
            return Err(LexerError::Lexer {
                message: "unexpected EOF".to_string(),
                line: token.line,
                column: token.column,
            })
        }

        let new_token = new_token.unwrap();

        if new_token.kind != LexerTokenType::String {
            return Err(LexerError::Lexer {
                message: format!("unexpected token {:?}", new_token.kind),
                line: new_token.line,
                column: new_token.column,
            })
        }

        let filename = &new_token.slice[1..new_token.slice.len() - 1];

        new_tokens.push(LexerToken {
            kind: LexerTokenType::EnterInclude,
            slice: Rc::from(filename),
            line: token.line,
            column: token.column,
        });

        println!("Including {}", filename);

        // include the file
        let mut file = fs::File::open(filename)
            .map_err(|e| LexerError::Other { error: Box::new(e) })?;

        let mut code = String::new();
        file.read_to_string(&mut code)
            .map_err(|e| LexerError::Other { error: Box::new(e) })?;
        drop(file);

        included.insert(filename.to_string(), code.clone());
        let code_borrowed = included.get(filename).unwrap();

        let mut tokens = tokenize(&code_borrowed)?;

        let mut preprocessor = Preprocessor::new(included);
        tokens = preprocessor.preprocess(tokens)?;

        new_tokens.append(&mut tokens);

        new_tokens.push(LexerToken {
            kind: LexerTokenType::ExitInclude,
            slice: Rc::from(filename),
            line: token.line,
            column: token.column,
        });

        Ok(())
    }
}