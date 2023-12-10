pub mod lexer;
pub mod parser;
pub mod symbols;
pub mod objgen;
pub mod linker;

use lexer::AsmLexer;
use parser::Parser;

use crate::{objgen::ObjectFormat, linker::Linker};

use std::{fs, env::args, process::ExitCode};

const VERSION: &'static str = "v0.1.0";
const GITHUB: &'static str = "https://github.com/pi4erd/sarch_asm";

fn print_version() {
    eprintln!("Sarch32 ASM Version {}\n{}", VERSION, GITHUB);
}

// TODO: Update with every argument
fn print_usage(program: &str) {
    eprintln!("\nUsage: {} <input_file>\n", program);
    eprintln!("\t-b | --oblect\t\t\tCompile to object without linking");
    eprintln!("\t-c | --link-script <filename>\tSpecify linker script");
    eprintln!("\t-h | --help\t\t\tPrint this menu");
    eprintln!("\t-k | --keep-object\t\tKeep an object file after linking");
    eprintln!("\t-o | --output <filename>\tSpecify output file");
    eprintln!("\t-v | --version\t\t\tPrint current version");
    eprintln!("\t-l | --link-object\t\tAdds object file to a linker");
    eprintln!("\t     --link\t\t\tTreat input file as SAO and link it");
}

fn main() -> ExitCode {
    // Debug stuff #
    let print_tokens = false;
    let print_ast = false;
    let print_object_tree = false;
    // ############

    let mut args: std::env::Args = args();

    // Inputs #####
    let mut input_file = String::new();
    let mut output_file = "output.bin".to_string();
    let mut linker_script: Option<&str> = None;
    let mut lib_files = Vec::<String>::new();
    let mut output_file_specified = false;
    let mut link_object = true;
    let mut input_is_object = false;
    let mut keep_object = false;
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
            }
            _ => {
                if input_file.is_empty() {
                    input_file = arg;
                    continue
                }
                print_usage(&program);
                return ExitCode::FAILURE
            }
        }
    }

    if input_file.is_empty() {
        print_usage(&program);
        return ExitCode::FAILURE
    }
    let mut objgenerator: ObjectFormat;

    if !input_is_object {
        let lexer = AsmLexer::new();

        let code = match fs::read_to_string(&input_file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read file: {}", e);
                return ExitCode::FAILURE
            }
        };
        
        let tokens = lexer.tokenize(&code);

        if print_tokens {
            for token in tokens.iter() {
                println!("Tokens: {:?}", token);
            }
        }

        let mut parser = Parser::new();
        let node = match parser.parse(&tokens) {
            Ok(n) => n,
            Err(err) => {
                eprintln!("Error occured while parsing:\n{}", err);
                return ExitCode::FAILURE
            }
        };

        if print_ast {
            println!("Parser tree: {:#?}", node);
        }

        objgenerator = ObjectFormat::new();
        match objgenerator.load_parser_node(node) {
            Ok(()) => {},
            Err(err) => {
                eprintln!("Error occured while generating object file:\n{}", err);
                return ExitCode::FAILURE
            }
        }
        if print_object_tree {
            println!("Object tree: {:#?}", objgenerator);
        }
    } else {
        objgenerator = match ObjectFormat::from_file(&input_file) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("Error occured while parsing binary from '{}': {}", input_file, e);
                return ExitCode::FAILURE
            }
        };
    }

    if keep_object {
        let (obj_file, _) = match input_file.rsplit_once('.') {
            Some(s) => s,
            None => (input_file.as_str(), "")
        };
        match objgenerator.save_object(&(obj_file.to_string() + ".sao")) {
            Ok(()) => {},
            Err(e) => {
                eprintln!("Error occured while saving binary into file:\n{}", e);
                return ExitCode::FAILURE
            }
        }
    }

    if link_object {
        let mut linker = Linker::new();
    
        match linker.load_symbols(objgenerator) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error occured while loading a symbol in linker: {e}");
                return ExitCode::FAILURE
            }
        };
        
        match linker.save_binary(&output_file, linker_script) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error occured while linking: {e}");
                return ExitCode::FAILURE
            }
        };
    }
    
    return ExitCode::SUCCESS
}
