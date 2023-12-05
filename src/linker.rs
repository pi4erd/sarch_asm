use crate::{objgen::{ObjectFormat, SectionData, InstructionData}, symbols::Instructions};
use std::{fs, io::{Write, Read}, collections::HashMap, ops::Index};
use byteorder::{LittleEndian, WriteBytesExt};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct LinkStructureSection {
    name: String,
    alignment: u64
}

#[derive(Debug, Serialize, Deserialize)]
struct LinkStructure {
    sections: Vec<LinkStructureSection>
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
                LinkStructureSection {
                    name: "text".to_string(),
                    alignment: 0x100
                },
                LinkStructureSection {
                    name: "data".to_string(),
                    alignment: 0x100
                },
                LinkStructureSection {
                    name: "rodata".to_string(),
                    alignment: 0x100
                },
            ]
        }
    }

    fn get_section(&self, name: &str) -> Option<&LinkStructureSection> {
        let mut sec_iter = self.sections.iter();

        sec_iter.find(|x| x.name == name)
    }

    fn get_section_index(&self, name: &str) -> Option<usize> {
        for (idx, sec) in self.sections.iter().enumerate() {
            if sec.name == name {
                return Some(idx)
            }
        }
        None
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

pub struct Linker {
    link_structure: LinkStructure,
    section_symbols: HashMap<String, SectionData>,
    section_binaries: HashMap<String, Vec<u8>>
}

impl Linker {
    pub fn new() -> Self {
        Self {
            link_structure: LinkStructure::new(),
            section_symbols: HashMap::new(),
            section_binaries: HashMap::new()
        }
    }

    pub fn load_symbols(&mut self, objfmt: ObjectFormat) -> Result<(), String> {
        for (sec_name, sec) in objfmt.sections {
            if self.section_symbols.contains_key(&sec_name) {
                self.section_symbols.get_mut(&sec_name).unwrap()
                    .append_other(sec)?;
            } else {
                self.section_symbols.insert(sec_name, sec);
            }
        }

        Ok(())
    }

    fn find_section_with_label(&self, label: &str) -> Option<&str> {
        let mut sec_iter = self.section_symbols.iter();

        // FIXME: This is messy. Maybe needs a refactor

        match sec_iter.find(|(_, x)| {
            if x.labels.contains_key(label) {
                return true
            }
            false
        }) {
            Some(s) => Some(s.0),
            None => None
        }
    }

    fn get_section_offset(&self, section_name: &str) -> Result<usize, String> {
        let link_section_index = match self.link_structure.get_section_index(section_name) {
            Some(lsi) => lsi,
            None => return Err(format!("Linker script doesn't define section '{}': Undefined reference", section_name))
        };

        let mut offset = 0usize;

        // For every section before this
        for (idx, link_section) in self.link_structure.sections.iter().enumerate() {
            if idx == link_section_index { break }
            let section = &self.section_symbols[&link_section.name];

            offset += section.get_binary_size();
        }

        

        Ok(offset)
    }

    fn write_instruction_binary(&self, binary: &mut Vec<u8>, instruction: &InstructionData) -> Result<(), String> {
        let instructions = Instructions::new();
        // Unwrap, because we assume valid section data from object files
        let instr_symbol = instructions.get_instruction(instruction.opcode).unwrap();

        let mut bin = Vec::<u8>::new();

        // Write opcode
        if instr_symbol.extended_opcode() {
            match bin.write_u16::<LittleEndian>(instr_symbol.opcode) {
                Ok(()) => {},
                Err(e) => {
                    return Err(format!("Failed to write binary: {e}"))
                }
            }
        } else {
            match bin.write_u8(instr_symbol.opcode as u8) {
                Ok(()) => {},
                Err(e) => {
                    return Err(format!("Failed to write binary: {e}"))
                }
            }
        }

        // Resolve symbols
        let mut resolved_references = Vec::<u32>::new();

        for reference in instruction.references.iter() {
            let sec_name = match self.find_section_with_label(&reference.rf) {
                Some(s) => s,
                None => {
                    return Err(format!("Failed to resolve reference '{}': Undefined reference", reference.rf))
                }
            };
            let section = &self.section_symbols[sec_name];

            let link_sec = match self.link_structure.get_section(sec_name) {
                Some(s) => s,
                None => {
                    return Err(format!("Linker script doesn't define information about section '{}'!", sec_name))
                }
            };

            
        }

        todo!()
    }

    fn section_binary(&self, binary: &mut Vec<u8>, section: &SectionData) -> Result<(), String> {
        if section.binary_section {
            binary.append(&mut section.binary_data.clone())
        } else {
            for instruction in section.instructions.iter() {
                self.write_instruction_binary(binary, instruction)?;
            }
        }
        todo!()
    }

    fn generate_binary(&mut self, ls_path: Option<&str>) -> Result<Vec<u8>, String> {
        self.link_structure = match ls_path {
            Some(lsp) => LinkStructure::from_file(lsp)?,
            None => LinkStructure::new()
        };

        for (sec_name, section) in self.section_symbols.iter() {
            let mut section_bin = Vec::<u8>::new();
            self.section_binary(&mut section_bin, section)?;
            self.section_binaries.insert(sec_name.clone(), section_bin);
        }

        let mut binary = Vec::<u8>::new();

        for (sec_name, sec_bin) in self.section_binaries.iter() {
            let section = match self.link_structure.get_section(sec_name) {
                Some(s) => s,
                None => {
                    return Err(format!("Cannot find section structure definition for '{sec_name}'."))
                }
            };
            
            
        }


        Ok(binary)
    }

    pub fn save_binary(&mut self, path: &str, ls_path: Option<&str>) -> Result<(), String> {
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
