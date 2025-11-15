use std::collections::HashMap;

use crate::lexer::{LexerError, LexerResult, LexerToken, LexerTokenType};

#[derive(Clone, Debug)]
struct Macro {
    args: Vec<String>,
    token_list: Vec<LexerToken>,
}

impl Macro {
    fn expand(&self, token: LexerToken, args: &[LexerToken]) -> LexerResult<Vec<LexerToken>> {
        // TODO: Multi-token per argument support
        if args.len() != self.args.len() {
            return Err(LexerError::Lexer {
                message: format!(
                    "Incorrect number of arguments for macro call: {} != {}",
                    args.len(), self.args.len()
                ),
                line: token.line,
                column: token.column,
            })
        }

        let mut tokens = Vec::new();

        for token in self.token_list.iter() {
            match token.kind {
                LexerTokenType::Escaped => {
                    let arg = self.args
                        .iter()
                        .enumerate()
                        .find(|(_, a)| *a == token.slice.as_ref());

                    if let Some((i, _)) = arg {
                        tokens.push(args[i].clone());
                    } else {
                        return Err(LexerError::EOF {
                            line: token.line,
                            column: token.column,
                        })
                    }
                },
                _ => tokens.push(token.clone()),
            }
        }

        Ok(tokens)
    }
}

pub struct Preprocessor<'a> {
    included: &'a mut HashMap<String, String>,
    macro_list: HashMap<String, Macro>,
}

impl<'a> Preprocessor<'a> {
    pub fn new(
        included: &'a mut HashMap<String, String>,
    ) -> Self {
        Self {
            included,
            macro_list: HashMap::new(),
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
                LexerTokenType::Identifier => {
                    if !self.macro_list.contains_key(token.slice.as_ref()) {
                        new.push(token);
                    } else {
                        let macro_name = token.slice.as_ref();

                        self.call_macro(macro_name, token.clone(), &mut new, &mut token_iter)?;
                    }
                }
                LexerTokenType::Comment => {}
                _ => new.push(token),
            }
        }

        Ok(new)
    }

    fn collect_arguments<I>(
        token: LexerToken,
        token_iter: &mut I,
    ) -> LexerResult<Vec<LexerToken>> where
        I: Iterator<Item = LexerToken>
    {
        let mut args = Vec::new();

        let mut last_token: Option<LexerToken> = None;

        while let Some(token) = token_iter.next() {
            args.push(token.clone());

            let token = token_iter.next()
                .ok_or(LexerError::EOF {
                    line: token.line,
                    column: token.column
                })?;
            last_token = Some(token.clone());
            
            if token.kind != LexerTokenType::Comma {
                break;
            }
        }

        last_token
            .ok_or(LexerError::EOF {
                line: token.line,
                column: token.column,
            })?
            .expect(LexerTokenType::RParen)?;

        Ok(args)
    }

    fn call_macro<I>(
        &self,
        macro_name: &str,
        token: LexerToken,
        new_tokens: &mut Vec<LexerToken>,
        token_iter: &mut I,
    ) -> LexerResult<()> where
        I: Iterator<Item = LexerToken>
    {
        let macro_def = &self.macro_list[macro_name];

        let token = token_iter.next()
            .ok_or(LexerError::EOF {
                line: token.line,
                column: token.column,
            })?;
        
        let args: Vec<LexerToken>;

        match token.kind {
            LexerTokenType::LParen => args = Self::collect_arguments(token.clone(), token_iter)?,
            _ => args = Vec::new(),
        }

        let mut tokens = macro_def.expand(token, &args)?;

        new_tokens.append(&mut tokens);

        Ok(())
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
            "macro" => {
                instructions::macro_definition(&mut self.macro_list, prev_token, token_iter)
            }
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
    use super::Macro;
    use std::{collections::HashMap, fs, io::Read, rc::Rc};

    use crate::{lexer::{LexerError, LexerResult, LexerToken, LexerTokenType, tokenize}, preprocessor::Preprocessor};

    fn collect_arguments<I>(
        token: LexerToken,
        token_iter: &mut I,
    ) -> LexerResult<Vec<String>> where
        I: Iterator<Item = LexerToken>
    {
        let mut args: Vec<String> = Vec::new();

        let mut last_token: Option<LexerToken> = None;

        while let Some(token) = token_iter.next() {
            match token.kind {
                LexerTokenType::Identifier => {
                    args.push(token.slice.to_string());
                    let token = token_iter.next()
                        .ok_or(LexerError::EOF {
                            line: token.line,
                            column: token.column,
                        })?;
                    last_token = Some(token.clone());
                    
                    if token.kind != LexerTokenType::Comma {
                        break;
                    }
                },
                _ => return Err(LexerError::Lexer {
                    message: format!("Unexpected token {:?}. {:?} expected.",
                        token.kind,
                        LexerTokenType::Identifier,
                    ),
                    line: token.line,
                    column: token.column,
                })
            }
        }

        last_token
            .ok_or(LexerError::EOF {
                line: token.line,
                column: token.column,
            })?.expect(LexerTokenType::RParen)?;

        return Ok(args)
    }

    fn collect_macro_tokens<I>(
        token_iter: &mut I
    ) -> LexerResult<Vec<LexerToken>> where 
        I: Iterator<Item = LexerToken>
    {
        let mut unmatched_brackets = 1;
        let mut tokens: Vec<LexerToken> = Vec::new();

        while let Some(token) = token_iter.next() {
            match token.kind {
                LexerTokenType::RBracket => {
                    unmatched_brackets -= 1;

                    if unmatched_brackets == 0 {
                        break;
                    }
                }
                LexerTokenType::LBracket => unmatched_brackets += 1,
                _ => {}
            }

            tokens.push(token);
        }

        Ok(tokens)
    }

    pub fn macro_definition<I>(
        macro_list: &mut HashMap<String, Macro>,
        token: LexerToken,
        token_iter: &mut I,
    ) -> LexerResult<()> where
        I: Iterator<Item = LexerToken>,
    {
        let token = token_iter.next()
            .ok_or(LexerError::EOF {
                line: token.line,
                column: token.column,
            })?;
        token.expect(LexerTokenType::Identifier)?;

        let macro_name = token.slice.to_string();

        let mut token = token_iter.next()
            .ok_or(LexerError::EOF {
                line: token.line,
                column: token.column,
            })?;
        
        let mut args: Option<Vec<String>> = None;
        let mut token_list: Vec<LexerToken>;

        if token.kind == LexerTokenType::LParen {
            args = Some(collect_arguments(token.clone(), token_iter)?);
            token = token_iter.next()
                .ok_or(LexerError::EOF {
                    line: token.line,
                    column: token.column,
                })?;
        }

        match token.kind {
            LexerTokenType::LBracket => {
                token_list = collect_macro_tokens(token_iter)?;
            }
            _ => return Err(LexerError::Lexer {
                message: format!("Unexpected token {:?}. {:?} or {:?} expected.",
                    token.kind,
                    LexerTokenType::LBracket, LexerTokenType::LParen,
                ),
                line: token.line,
                column: token.column,
            })
        }

        token_list = token_list
            .into_iter()
            .filter(|t| t.kind != LexerTokenType::Comment)
            .collect::<Vec<_>>();

        let macro_def = Macro {
            args: args.unwrap_or(Vec::new()),
            token_list
        };

        macro_list.insert(macro_name, macro_def);
        
        Ok(())
    }

    pub fn include<I>(
        included: &mut HashMap<String, String>,
        new_tokens: &mut Vec<LexerToken>,
        token: LexerToken,
        token_iter: &mut I,
    ) -> LexerResult<()> where
        I: Iterator<Item = LexerToken>
    {
        // TODO: Fix recursive includes

        let new_token = token_iter.next();

        if new_token.is_none() {
            return Err(LexerError::EOF {
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
