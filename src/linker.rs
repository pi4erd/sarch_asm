use crate::objgen::ObjectFormat;
use std::{fs, io::{Write, Read}, collections::HashMap};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct LinkSection {
    name: String,
    alignment: u64
}

#[derive(Debug, Serialize, Deserialize)]
struct LinkStructure {
    sections: Vec<LinkSection>
}

impl LinkStructure {
    /**
     * Creates a default link structure
     * 
     * Default structure includes sections: text, data, rodata (ordered)
     * All sections by default are aligned to 0x100 bytes in hex
     */
    fn new() -> Self {
        Self {
            sections: vec![
                LinkSection {
                    name: "text".to_string(),
                    alignment: 0x100
                },
                LinkSection {
                    name: "data".to_string(),
                    alignment: 0x100
                },
                LinkSection {
                    name: "rodata".to_string(),
                    alignment: 0x100
                },
            ]
        }
    }

    fn from_file(path: &str) -> Result<Self, String> {
        let mut file = match fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                return Err(format!("Failed to open file '{}' for reading!\n{}", path, e))
            }
        };

        let mut txt = String::new();

        match file.read_to_string(&mut txt) {
            Ok(l) => l,
            Err(e) => {
                return Err(format!("Error reading file '{}': {}", path, e))
            }
        };

        Self::from_text(txt)
    }

    fn from_text(txt: String) -> Result<Self, String> {
        let link_struct = match serde_json::from_str::<LinkStructure>(&txt) {
            Ok(l) => l,
            Err(e) => {
                return Err(format!("Error occured while parsing JSON: {e}"))
            }
        };
        Ok(link_struct)
    }
}

struct SectionBinary {
    binary: Vec<u8>
}

pub struct Linker {
    objects: Vec<ObjectFormat>, // used for symbol resolving
    binaries: HashMap<String, SectionBinary>
}

impl Linker {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            binaries: HashMap::new()
        }
    }

    pub fn load_symbols(&mut self, objfmt: ObjectFormat) {
        self.objects.push(objfmt);
    }

    fn generate_binary(&self, ls_path: Option<&str>) -> Result<Vec<u8>, String> {
        let link_struct = match ls_path {
            Some(lsp) => LinkStructure::from_file(lsp)?,
            None => LinkStructure::new()
        };
         
        todo!()
    }

    pub fn save_binary(&self, path: &str, ls_path: Option<&str>) -> Result<(), String> {
        let bin = self.generate_binary(ls_path)?;

        let mut file = match fs::File::create(path) {
            Ok(f) => f,
            Err(e) => {
                return Err(format!("Error occured while trying to open file for saving: {e}"))
            }
        };

        match file.write_all(bin.as_slice()) {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(format!("Error occured while writing binary to file: {e}"))
            }
        }
    }
}
