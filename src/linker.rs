use crate::{objgen::{ObjectFormat, SectionData, InstructionData, ConstantSize}, symbols::{Instructions, ArgumentTypes}};
use std::{fs, io::{Write, Read}, collections::HashMap};
use byteorder::{LittleEndian, WriteBytesExt};
use serde::{Serialize, Deserialize};

macro_rules! calculate_alignment {
    ($num:expr, $alignment:expr) => {
        // TODO: Optimize macro
        if $num > ($num / $alignment) * $alignment {
            ($num / $alignment) * $alignment + $alignment
        } else {
            ($num / $alignment) * $alignment
        }
    };
}

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

struct ResolvedReference {
    size: ConstantSize,
    value: i64
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

    fn get_section_offset(&self, section_name: &str) -> Result<u64, String> {
        let link_section_index = match self.link_structure.get_section_index(section_name) {
            Some(lsi) => lsi,
            None => return Err(format!("Linker script doesn't define section '{}': Undefined reference.", section_name))
        };

        let mut offset = 0u64;

        // For every section before this
        for (idx, link_section) in self.link_structure.sections.iter().enumerate() {
            if idx == link_section_index { break }
            let section = &self.section_symbols[&link_section.name];

            offset += section.get_binary_size() as u64;
        }

        let alignment = self.link_structure.get_section(section_name)
            .unwrap().alignment;

        let result = calculate_alignment!(offset, alignment);

        Ok(result)
    }

    fn write_instruction_binary(&self, binary: &mut Vec<u8>, instruction: &InstructionData) -> Result<(), String> {
        let instructions = Instructions::new();
        // Unwrap, because we assume valid section data from object files
        let instr_symbol = instructions.get_instruction(instruction.opcode).unwrap();

        let start_position = binary.len() as i64;

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
        let mut resolved_references = HashMap::<u8, ResolvedReference>::new();

        for reference in instruction.references.iter() {
            let sec_name = match self.find_section_with_label(&reference.rf) {
                Some(s) => s,
                None => {
                    return Err(format!("Failed to resolve reference '{}': Undefined reference.", reference.rf))
                }
            };
            let section = &self.section_symbols[sec_name];

            // Unwrap because previous statement, read it again pls;;;
            let section_local_offset = section.get_label_binary_offset(&reference.rf).unwrap();

            let section_offset = self.get_section_offset(sec_name)?;

            let offset = section_offset + section_local_offset;

            let arg_size = instr_symbol.args[reference.argument_pos as usize].get_size();

            // FIXME: Unwraps
            resolved_references.insert(reference.argument_pos, ResolvedReference { 
                size: ConstantSize::from_u8(arg_size as u8).unwrap(), value: offset as i64 
            });
        }

        for constant in instruction.constants.iter() {
            resolved_references.insert(constant.argument_pos, ResolvedReference {
                size: constant.size, value: constant.value
            });
        }
        
        // FIXME: Actually i am stupid and have no idea how to do this otherwise.
        // If anyone has any idea on how to improve this piece of... code...
        // Please help me. I would appreciate any direction anyone is willing to give me.

        // Why do i have to borrow a ZERO?
        if let Some(arg) = resolved_references.get_mut(&0) {
            let sym_arg = instr_symbol.args[0];
            match sym_arg {
                // Calculate relative offset
                ArgumentTypes::RelPointer => {
                    arg.value = arg.value - start_position;
                }
                _ => {}
            }
            match arg.size {
                // FIXME: UNWRAPS
                ConstantSize::Byte => bin.write_i8(arg.value as i8).unwrap(),
                ConstantSize::Word => bin.write_i16::<LittleEndian>(arg.value as i16).unwrap(),
                ConstantSize::DoubleWord => bin.write_i32::<LittleEndian>(arg.value as i32).unwrap()
            }
        }
        // instructions are packed, and not aligned, so it should be fine to do this, right?
        if let Some(arg) = resolved_references.get_mut(&1) {
            let sym_arg = instr_symbol.args[1];
            match sym_arg {
                ArgumentTypes::RelPointer => {
                    arg.value = arg.value - start_position;
                }
                _ => {}
            }
            match arg.size {
                // FIXME: UNWRAPS
                ConstantSize::Byte => bin.write_i8(arg.value as i8).unwrap(),
                ConstantSize::Word => bin.write_i16::<LittleEndian>(arg.value as i16).unwrap(),
                ConstantSize::DoubleWord => bin.write_i32::<LittleEndian>(arg.value as i32).unwrap()
            }
        }

        binary.append(&mut bin);

        Ok(())
    }

    fn section_binary(&self, binary: &mut Vec<u8>, section: &SectionData) -> Result<(), String> {
        if section.binary_section {
            binary.append(&mut section.binary_data.clone());
        } else {
            for instruction in section.instructions.iter() {
                self.write_instruction_binary(binary, instruction)?;
            }
        }

        Ok(())
    }

    pub fn generate_binary(&mut self, ls_path: Option<&str>) -> Result<Vec<u8>, String> {
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

        for section in self.link_structure.sections.iter() {
            if let Some(mut bin) = self.section_binaries.get_mut(&section.name) {
                binary.append(&mut bin);
            } else {
                return Err(format!("Undefined reference to section '{}': \
                linker section is defined but not found in binaries!", section.name))
            }

            let offset = self.get_section_offset(&section.name)?;
            let end = offset + self.section_symbols[&section.name].get_binary_size() as u64;

            let alignment_bit_count = calculate_alignment!(end, section.alignment) - end;

            // God forgive me
            for _ in 0..alignment_bit_count {
                binary.push(0);
            }
        }

        Ok(binary)
    }

    pub fn save_binary(&mut self, path: &str, ls_path: Option<&str>) -> Result<(), String> {
        println!("Loaded symbols: {:#?}", self.section_symbols);

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
