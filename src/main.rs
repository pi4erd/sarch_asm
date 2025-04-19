pub mod preprocessor;
pub mod lexer;
pub mod parser;
pub mod symbols;
pub mod objgen;
pub mod linker;
pub mod objdump;

pub mod tests;

use lexer::{AsmLexer, LexerToken};
use objdump::Objdump;
use parser::{Parser, ParserNode};
use preprocessor::Preprocessor;
use regex_lexer::Token;

use crate::{objgen::ObjectFormat, linker::Linker};

use std::{fs, env::args, process::ExitCode};

const VERSION: &'static str = env!("CARGO_PKG_VERSION", "No crate version is defined in environment variables.");
const GITHUB: &'static str = "https://github.com/pi4erd/sarch_asm";

fn print_version() {
    eprintln!("Sarch32 ASM Version {}\n{}", VERSION, GITHUB);
}

// TODO: Update with every argument
fn print_usage(program: &str) {
    eprintln!("\nUsage: {} <input_file>\n", program);
    eprintln!("\t-b | --oblect\t\t\tCompile to object without linking");
    eprintln!("\t-c | --link-script <filename>\tSpecify linker script");
    eprintln!("\t-d | --disassemble\t\tToggle disassembly for an object file");
    eprintln!("\t-h | --help\t\t\tPrint this menu");
    eprintln!("\t-k | --keep-object\t\tKeep an object file after linking");
    eprintln!("\t-o | --output <filename>\tSpecify output file");
    eprintln!("\t-v | --version\t\t\tPrint current version");
    eprintln!("\t-l | --link-object\t\tAdds object file to a linker");
    eprintln!("\t     --entrypoint\t\tSpecify entrypoint of a program");
    eprintln!("\t     --link\t\t\tTreat input file as SAO and link it");
}

pub fn preprocess(code: String) -> Result<String, String> {
    let pp = Preprocessor::new(code);
    pp.preprocess()
}

pub fn lex(code: &str, print_tokens: bool) -> Vec<Token<'_, LexerToken>> {
    let lexer = AsmLexer::new();
    let tokens = lexer.tokenize(&code);

    if print_tokens {
        for token in tokens.iter() {
            println!("Tokens: {:?}", token);
        }
    }

    tokens
}

pub fn parse(tokens: Vec<Token<'_, LexerToken>>, print_ast: bool) -> Result<ParserNode, String> {
    let mut parser = Parser::new();
    match parser.parse(&tokens) {
        Ok(n) => n,
        Err(err) => {
            return Err(format!("Error occured while parsing:\n{}", err))
        }
    };

    if print_ast {
        println!("Parser tree: {:#?}", &parser.root);
    }

    Ok(parser.root)
}

fn main() -> ExitCode {
    // Debug stuff #
    let print_tokens = false;
    let print_ast = false;
    let print_object_tree = false;
    // ############

    let mut args: std::env::Args = args();

    // Inputs #####
    let mut input_files: Vec<String> = Vec::new();
    let mut output_file = "output.bin".to_string();
    let mut linker_script: Option<&str> = None;
    let mut lib_files = Vec::<String>::new();
    let mut output_file_specified = false;
    let mut link_object = true;
    let mut input_is_object = false;
    let mut keep_object = false;
    let mut disassemble = false;
    let mut print_resolve_sections = false;
    let mut entrypoint: Option<String> = None;
    // ############

    let mut linker_script_filename: String;

    let program = args.next().unwrap();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                if output_file_specified {
                    eprintln!("Unable to specify multiple output files ('-o' flags)");
                    print_usage(&program);
                    return ExitCode::FAILURE;
                }
                let filename = match args.next() {
                    Some(f) => f,
                    None => {
                        eprintln!("Expected filename after '-o'.");
                        print_usage(&program);
                        return ExitCode::FAILURE;
                    }
                };
                output_file = filename;
                output_file_specified = true;
            }
            "-h" | "--help" => {
                print_usage(&program);
                return ExitCode::SUCCESS
            }
            "-v" | "--version" => {
                print_version();
                return ExitCode::SUCCESS
            }
            "-k" | "--keep-object" => {
                keep_object = true;
                link_object = true;
            }
            "-b" | "--object" => {
                keep_object = true;
                link_object = false;
            }
            "-c" | "--link-script" => {
                if linker_script != None {
                    eprintln!("Cannot specify multiple linker scripts!");
                    print_usage(&program);
                    return ExitCode::FAILURE
                }
                linker_script_filename = match args.next() {
                    Some(f) => f,
                    None => {
                        eprintln!("Expected filename after '{}'.", arg);
                        print_usage(&program);
                        return ExitCode::FAILURE;
                    }
                };
                linker_script = Some(&linker_script_filename);
            }
            "-d" | "--disassemble" => {
                disassemble = true;
                input_is_object = true;
            }
            "-l" | "--link-object" => {
                // Adds object file to the linker
                // Like -l in GNUC, it links binary object files

                let filename = match args.next() {
                    Some(f) => f,
                    None => {
                        eprintln!("Expected filename after '{}'", arg);
                        print_usage(&program);
                        return ExitCode::FAILURE
                    }
                };
                lib_files.push(filename);
            }
            "--link" => {
                // Links input file as object file without compiling it
                // May be useful trying to compile multiple object files
                input_is_object = true;
                link_object = true;
            }
            "--resolve-sections" => {
                // Prints all sections and their corresponding addresses
                // for binary files
                input_is_object = true;
                link_object = true;
                print_resolve_sections = true;
            }
            "--entrypoint" => {
                let labelname = match args.next() {
                    Some(lbl) => lbl,
                    None => {
                        eprintln!("Expected label name after '{arg}'");
                        print_usage(&program);
                        return ExitCode::FAILURE
                    }
                };
                entrypoint = Some(labelname)
            }
            _ => {
                input_files.push(arg);
            }
        }
    }

    if input_files.len() == 0 {
        print_usage(&program);
        return ExitCode::FAILURE
    }
    let mut objects: Vec<ObjectFormat> = Vec::new();

    if !input_is_object {
        for filepath in input_files.iter() {
            let code = match fs::read_to_string(filepath) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to read file: {}", e);
                    return ExitCode::FAILURE
                }
            };

            let new_code = match preprocess(code) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to preprocess file: {e}");
                    return ExitCode::FAILURE
                }
            };
            
            let tokens = lex(&new_code, print_tokens);

            let node = match parse(tokens, print_ast) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("{}", e);
                    return ExitCode::FAILURE
                }
            };

            let mut object = ObjectFormat::new();
            match object.load_parser_node(&node) {
                Ok(()) => {},
                Err(err) => {
                    eprintln!("Error occured while generating object file:\n{}", err);
                    return ExitCode::FAILURE
                }
            }
            if print_object_tree {
                println!("Object tree: {:#?}", object);
            }

            objects.push(object)
        }
    }
    else {
        for object_input in input_files.iter() {
            let object = match ObjectFormat::from_file(object_input) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error occured while parsing binary from '{}': {}", object_input, e);
                    return ExitCode::FAILURE
                }
            };
            objects.push(object)
        }
    }

    if disassemble {
        if objects.len() > 1 {
            eprintln!("Cannot disassemble multiple files!");
            return ExitCode::FAILURE
        }
        let object = match objects.get(0) {
            Some(o) => o,
            None => {
                eprintln!("Not enough object files!");
                print_usage(&program);
                return ExitCode::FAILURE
            }
        };
        let input_file = &input_files[0];
        let dumper = Objdump::new(object.clone());
        match dumper.get_disassembly() {
            Ok(s) => {
                println!("Disassembly for '{}':\n", input_file);
                println!("{}", s);
            }
            Err(e) => {
                eprintln!("Error occured while disassembling file: {e}");
                return ExitCode::FAILURE
            }
        }
        return ExitCode::SUCCESS;
    }

    if keep_object && !link_object {
        if input_files.len() > 1 {
            eprintln!("Cannot compile multiple object files without linking!");
            print_usage(&program);
            return ExitCode::FAILURE
        }
        let object = &objects[0];
        match object.save_object(&output_file) {
            Ok(()) => {},
            Err(e) => {
                eprintln!("Error occured while saving binary into file:\n{}", e);
                return ExitCode::FAILURE
            }
        }
        return ExitCode::SUCCESS
    }

    if link_object {
        let mut linker = Linker::new();

        if let Some(entry_label) = entrypoint {
            let first_object = ObjectFormat::create_jumper(entry_label);
            match linker.load_symbols(first_object) {
                Ok(_) => {},
                Err(e) => {
                    // this error shouldn't happen. if it does happen,
                    // then please fix this in objgen.rs/ObjectFormat::create_jumper()
                    eprintln!("Compiler error occured (you're lucky): {e}");
                    return ExitCode::FAILURE
                }
            };
        }
    
        for object in objects {
            match linker.load_symbols(object) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error occured while loading a symbol in linker: {e}");
                    return ExitCode::FAILURE
                }
            };
        }
        
        for lib in lib_files {
            let lib_fmt = match ObjectFormat::from_file(&lib) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Error occured while reading library object: {e}");
                    return ExitCode::FAILURE
                }
            };
            match linker.load_symbols(lib_fmt) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error occured while loading a library in linker: {e}");
                    return ExitCode::FAILURE
                }
            };
        }

        if keep_object {
            let filename = output_file.clone() + ".sao";

            match linker.save_object(&filename) {
                Ok(()) => {},
                Err(e) => {
                    eprintln!("Error occured while saving linker object: {e}");
                    return ExitCode::FAILURE
                }
            }
        }

        match linker.save_binary(&output_file, linker_script) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error occured while linking: {e}");
                return ExitCode::FAILURE
            }
        };

        if print_resolve_sections {
            println!("{}", match linker.list_resolve_sections() {
                Ok(s) => s,
                Err(e) => format!("Unable to list sections: {e}"),
            });
        }
    }
    
    return ExitCode::SUCCESS
}
