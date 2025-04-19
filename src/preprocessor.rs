use std::collections::HashSet;

use regex::Regex;

#[derive(Debug)]
struct Macro {
    name: String,
    replacement: String
}

pub struct Preprocessor {
    code: String,
}

// Preprocessor directive prefix: '%'
impl Preprocessor {
    pub fn new(code: String) -> Self {
        Self { code }
    }

    fn rid_comments(code: String) -> String {
        let mut result = String::new();

        let mut inside_comment = false;

        for c in code.chars() {
            if inside_comment {
                if c == '\n' {
                    inside_comment = false;
                    result.push(c);
                }
                continue
            }

            if c == ';' || c == '#' {
                inside_comment = true;
                continue;
            }

            result.push(c);
        }

        result
    }

    fn parse_macro_definition(code: &str, start_pos: usize) -> Result<Macro, String> {
        #[derive(PartialEq)]
        enum ParseState {
            End, Name, Content
        }

        const WHITESPACE_CHARS: &str = " \t\n\r";
        let allowed_name_rgx = Regex::new(r"(?:\w|[_])*").map_err(|e| format!("{e}"))?;

        let mut macro_name = String::new();
        let mut macro_content = String::new();

        let mut constructing_name = false;
        let mut constructing_content = false;

        let mut current_state = ParseState::Name;

        let mut str_iter = code.chars().skip(start_pos);

        'parse_loop: while let Some(current_char) = str_iter.next() {
            match current_state {
                ParseState::Name => {
                    if WHITESPACE_CHARS.contains(current_char) {
                        if !constructing_name { continue }
                        else {
                            constructing_name = false;
                            current_state = ParseState::Content;
                            continue
                        }
                    }

                    if !allowed_name_rgx.is_match(&format!("{current_char}")) {
                        constructing_name = false;
                        current_state = ParseState::Content;
                        continue
                    }

                    if !constructing_name {
                        constructing_name = true;
                    }
                    macro_name.push(current_char);
                }
                ParseState::Content => {
                    if WHITESPACE_CHARS.contains(current_char) {
                        if !constructing_content { continue }
                    }

                    if current_char == '{' {
                        if constructing_content {
                            return Err(format!("Syntax error: double opening brace inside macro"))
                        }

                        constructing_content = true;
                        continue
                    }

                    if current_char == '}' {
                        if !constructing_content {
                            return Err(format!("Syntax error: closing brace unmatched inside macro"))
                        }

                        constructing_content = false;
                        current_state = ParseState::End;
                        continue;
                    }

                    macro_content.push(current_char);
                }
                ParseState::End => {
                    if constructing_name || constructing_content {
                        return Err(format!("Syntax error while parsing macro '{}'. Check for unmatched braces.", macro_name))
                    }
                    break 'parse_loop
                }
            }
        }

        if current_state != ParseState::End {
            return Err(format!("Unknown error occured while parsing macro '{}'.", macro_name))
        }

        Ok(Macro {
            name: macro_name,
            replacement: macro_content
        })
    }

    fn find_macro_definitions(code: &str) -> Result<(Vec<Macro>, String), String> {
        let mut macros = Vec::new();

        let mut macro_check: HashSet<String> = HashSet::new();

        let macro_rgx = Regex::new(r"\%macro").map_err(|e| format!("Regex error: {e}"))?;
        let macro_repl_rgx = Regex::new(r"\%macro\s+(\w|[_])+\s*\{(.|\s)*\}").map_err(|e| format!("Regex error: {e}"))?;

        for macro_def_match in macro_rgx.find_iter(code) {
            let macro_def = Self::parse_macro_definition(code, macro_def_match.end())?;

            if macro_check.contains(&macro_def.name) {
                return Err(format!("Redefinition of macro '{}'", macro_def.name));
            }

            macro_check.insert(macro_def.name.clone());
            macros.push(
                macro_def
            );
        }

        let new_code = macro_repl_rgx.replace_all(code, "").to_string();
        
        Ok((macros, new_code))
    }

    fn find_and_process_macro_calls(code: &str, macros: Vec<Macro>) -> Result<String, String> {
        let mut result = code.to_string();

        for macro_ in macros.iter() {
            // this is gross but bear with it
            let call_rgx_str = format!(r"\%call\s+{}", macro_.name);
            let call_rgx = Regex::new(&call_rgx_str)
                .map_err(|e| format!("Regex error occured while generating regex for macro: {e}"))?;

            // Ew
            result = call_rgx.replace_all(&result, &macro_.replacement).to_string();
        }

        Ok(result)
    }

    fn process_includes(mut code: String) -> Result<String, String> {
        let include_regex: Regex = Regex::new(r#"%include\s"(.+)""#).unwrap();
        let new_code = code.clone();
        let captures = include_regex.captures_iter(&new_code);

        for cap in captures {
            let filename = cap.get(1).unwrap().as_str();
            let full_statement = cap.get(0).unwrap().as_str();

            let file_bytes = std::fs::read(filename)
                .map_err(|e| format!("Failed to read file included '{filename}': {e}"))?;
            let file_text = String::from_utf8(file_bytes)
                .map_err(|e| format!("Failed to convert to utf8: {e}"))?;

            code = code.replace(full_statement, &format!("{}\n", file_text));
        }

        Ok(code)
    }

    pub fn preprocess(mut self) -> Result<String, String> {
        self.code = Self::rid_comments(self.code);
        self.code = Self::process_includes(self.code)?;

        let (macros, code) = Self::find_macro_definitions(&self.code)?;
        self.code = code;

        self.code = Self::find_and_process_macro_calls(&self.code, macros)?;

        Ok(self.code)
    }
}
