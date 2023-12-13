/**
 * objgen.rs
 * 
 * Generates object files for SArch32 ASM. Default extension: .sao
 */

use std::collections::HashMap;
use std::io::{Error, Write};
use std::{fs, io, str};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::parser::{ParserNode, NodeType, Registers};
use crate::symbols::{Instructions, ArgumentTypes, Conditions};

macro_rules! unexpected_node {
    ($node:expr) => {
        return Err(format!("Unexpected node {:?}!", $node.node_type))
    };
}
macro_rules! wrong_argument {
    ($node:expr, $expected:expr) => {
        return Err(format!("Incorrect argument of {:?}. {:?} expected.", $node.node_type, $expected))
    };
}
macro_rules! bad_compinstr {
    ($iname:expr) => {
        return Err(format!("Invalid compiler instruction '{}'. No such instruction exists!", $iname))
    };
}
macro_rules! unexpected_eof {
    ($msg:expr) => {
        return Err(format!("Unexpected end of file: {}", $msg))
    };
}

const MAGIC_FORMAT_NUMBER: u64 = 0x3A6863FC6173371B;
const CURRENT_FORMAT_VERSION: u32 = 3;

/**
 * 0 - 1: argument position
 * 1 - <>: reference name
 */
#[derive(Debug, Clone)]
pub struct Reference {
    pub argument_pos: u8,
    pub rf: String
}

impl Reference {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            argument_pos: 0,
            rf: String::new()
        };

        me.argument_pos = binary.read_u8()?;

        let mut char_vec = Vec::<u8>::new();

        let mut c = binary.read_u8()?;

        while c != 0 {
            char_vec.push(c);
            c = binary.read_u8()?;
        }

        me.rf = String::from_utf8(char_vec).unwrap();

        Ok(me)
    }
    fn write_bytes(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        binary.write_u8(self.argument_pos)?;

        for c in self.rf.bytes() {
            binary.write_u8(c)?;
        }
        binary.write_u8(0)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConstantSize {
    Byte, Word, DoubleWord
}

impl ConstantSize {
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            1 => Some(ConstantSize::Byte),
            2 => Some(ConstantSize::Word),
            4 => Some(ConstantSize::DoubleWord),
            _ => None
        }
    }
    fn to_u8(&self) -> u8 {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
            Self::DoubleWord => 4
        }
    }
    pub fn get_size(&self) -> usize {
        self.to_u8() as usize
    }
}

/**
 * 0 - 1: argument position
 * 1 - 2: const size
 * 2 - 10: value
 */
#[derive(Debug, Clone)]
pub struct Constant {
    pub argument_pos: u8,
    pub size: ConstantSize,
    pub value: i64
}

impl Constant {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            argument_pos: 0,
            size: ConstantSize::Byte,
            value: 0
        };

        me.argument_pos = binary.read_u8()?;

        me.size = match ConstantSize::from_u8(binary.read_u8()?) {
            Some(n) => n,
            None => {
                return Err(Error::new(io::ErrorKind::InvalidData,
                format!("Wrong constant size in instruction!")))
            }
        };

        me.value = match me.size {
            ConstantSize::Byte => binary.read_i8()? as i64,
            ConstantSize::Word => binary.read_i16::<LittleEndian>()? as i64,
            ConstantSize::DoubleWord => binary.read_i32::<LittleEndian>()? as i64,
        };

        Ok(me)
    }
    fn write_bytes(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        binary.write_u8(self.argument_pos)?;
        binary.write_u8(self.size.to_u8())?;

        match self.size {
            ConstantSize::Byte => binary.write_i8(self.value as i8),
            ConstantSize::Word => binary.write_i16::<LittleEndian>(self.value as i16),
            ConstantSize::DoubleWord => binary.write_i32::<LittleEndian>(self.value as i32)
        }?;

        Ok(())
    }
}

/**
 * 0 - 2: opcode
 * 2 - 3: reference count
 * 3 - 4: constant count
 * 4 - <>: references
 * <> - <>: constants
 */

#[derive(Debug, Clone)]
pub struct InstructionData {
    pub opcode: u16,
    pub references: Vec<Reference>,
    pub constants: Vec<Constant>
}

impl InstructionData {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            opcode: 0xFFFF,
            references: Vec::new(),
            constants: Vec::new()
        };

        me.opcode = binary.read_u16::<LittleEndian>()?;
        let ref_count = binary.read_u8()?;
        let const_count = binary.read_u8()?;

        for _ in 0..ref_count {
            let reference = Reference::from_bytes(binary)?;
            me.references.push(reference);
        }

        for _ in 0..const_count {
            let constant = Constant::from_bytes(binary)?;
            me.constants.push(constant);
        }

        // FIXME: Is there a better way to do this check?
        for rf in me.references.iter() {
            for cst in me.constants.iter() {
                if cst.argument_pos == rf.argument_pos {
                    return Err(
                        Error::new(io::ErrorKind::InvalidData, 
                            format!("Reference and constant are pointing to same argument space. Maybe file corrupted?")))
                }
            }
        }

        Ok(me)
    }
    fn write_bytes(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        binary.write_u16::<LittleEndian>(self.opcode)?;
        binary.write_u8(self.references.len() as u8)?;
        binary.write_u8(self.constants.len() as u8)?;
        
        for rf in self.references.iter() {
            rf.write_bytes(binary)?;
        }

        for cst in self.constants.iter() {
            cst.write_bytes(binary)?;
        }

        Ok(())
    }
}

/**
 * 0 - 8: ptr instr
 * 8 - 16: ptr bin
 * 16 - <>: name
 */
#[derive(Debug, Clone)]
pub struct ObjectLabelSymbol {
    name: String,
    ptr_instr: u64,
    ptr_binary: u64,
}

impl ObjectLabelSymbol {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            name: String::new(),
            ptr_instr: 0,
            ptr_binary: 0
        };

        me.ptr_instr = binary.read_u64::<LittleEndian>()?;
        me.ptr_binary = binary.read_u64::<LittleEndian>()?;

        let mut char_vec = Vec::<u8>::new();

        let mut c = binary.read_u8()?;

        while c != 0 {
            char_vec.push(c);
            c = binary.read_u8()?;
        }

        me.name = String::from_utf8(char_vec).unwrap();

        Ok(me)
    }
    fn write_bytes(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        binary.write_u64::<LittleEndian>(self.ptr_instr)?;
        binary.write_u64::<LittleEndian>(self.ptr_binary)?;

        for b in self.name.bytes() {
            binary.write_u8(b)?;
        }
        binary.write_u8(0)?;

        Ok(())
    }
}

/**
 * Section structure description:
 * 0 - 8: instruction count
 * 8 - 16: label count
 * 16 - 24: binary size
 * 24 - <>: section name
 * <> - <>: Labels
 * <> - <>: Instructions
 * <> - <>: Binary
 */
#[derive(Debug, Clone)]
pub struct SectionData {
    name: String,
    pub instructions: Vec<InstructionData>,
    pub labels: HashMap<String, ObjectLabelSymbol>,
    pub binary_data: Vec<u8>,
    pub binary_section: bool
}

impl SectionData {
    fn new() -> Self {
        Self {
            name: "text".to_string(),
            instructions: Vec::new(),
            labels: HashMap::new(),
            binary_data: Vec::new(),
            binary_section: false
        }
    }
    pub fn append_other(&mut self, mut other: SectionData) -> Result<(), String> {
        if self.binary_section != other.binary_section {
            return Err(format!("Cannot merge binary section with non-binary one"))
        }
        if self.binary_section {
            let old_bin_length = self.binary_data.len() as u64;
            self.binary_data.append(&mut other.binary_data);
            
            for (label_name, mut label) in other.labels {
                if self.labels.contains_key(&label_name) {
                    return Err(format!("Cannot merge two binary sections with similar labels!"))
                }
                label.ptr_binary += old_bin_length;
                self.labels.insert(label_name, label);
            }
        } else {
            let old_instr_length = self.instructions.len() as u64;
            self.instructions.append(&mut other.instructions);
            
            for (label_name, mut label) in other.labels {
                if self.labels.contains_key(&label_name) {
                    return Err(format!("Cannot merge two binary sections with similar labels!"))
                }
                label.ptr_instr += old_instr_length;
            }
        }

        Ok(())
    }

    pub fn get_binary_size(&self) -> usize {
        if self.binary_section {
            return self.binary_data.len()
        }

        let instructions = Instructions::new();

        let mut binary_len = 0usize;

        for instr in self.instructions.iter() {
            // Unwrap, because we assume a section is valid from object file
            binary_len += instructions.get_instruction(instr.opcode).unwrap().get_size();
        }

        binary_len
    }

    pub fn get_binary_position(&self, index: u64) -> u64 {
        if self.binary_section {
            return index
        }

        let instructions = Instructions::new();

        let mut binary_index = 0u64;

        for (idx, instr) in self.instructions.iter().enumerate() {
            if idx as u64 == index { break }
            // I won't explain why I'm adding unwraps anymore
            binary_index += instructions.get_instruction(instr.opcode).unwrap().get_size() as u64;
        }

        binary_index
    }

    pub fn get_label_binary_offset(&self, label_name: &str) -> Option<u64> {
        let label = self.labels.get(label_name)?;

        if self.binary_section { return Some(label.ptr_binary) }

        Some(self.get_binary_position(label.ptr_instr))
    }

    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self::new();

        let instruction_count = binary.read_u64::<LittleEndian>()?;
        let label_count = binary.read_u64::<LittleEndian>()?;
        let binary_count = binary.read_u64::<LittleEndian>()?;

        let mut char_vec = Vec::<u8>::new();

        let mut c = binary.read_u8()?;

        while c != 0 {
            char_vec.push(c);
            c = binary.read_u8()?;
        }

        me.name = String::from_utf8(char_vec).unwrap();

        for _ in 0..label_count {
            let label = ObjectLabelSymbol::from_bytes(binary)?;

            let name = label.name.clone();

            if me.labels.contains_key(&name) {
                return Err(Error::new(io::ErrorKind::InvalidData,
                format!("Invalid label information for section '{}'! Label '{}' already exists!",
                me.name, name)))
            }

            me.labels.insert(label.name.clone(), label);
        }

        for _ in 0..instruction_count {
            let instruction = InstructionData::from_bytes(binary)?;
            me.instructions.push(instruction);
        }

        for _ in 0..binary_count {
            let binary = binary.read_u8()?;
            me.binary_data.push(binary);
        }

        me.binary_section = me.binary_data.len() != 0;

        Ok(me)
    }
    fn write_bytes(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        if self.binary_data.len() != 0 && self.instructions.len() != 0 {
            return Err(Error::new(io::ErrorKind::InvalidInput,
                format!("Binary and instructions cannot coexist in a single section!")))
        }

        binary.write_u64::<LittleEndian>(self.instructions.len() as u64)?;
        binary.write_u64::<LittleEndian>(self.labels.len() as u64)?;
        binary.write_u64::<LittleEndian>(self.binary_data.len() as u64)?;

        for b in self.name.bytes() {
            binary.write_u8(b)?;
        }
        binary.write_u8(0)?;

        for (_, lbl) in self.labels.iter() {
            lbl.write_bytes(binary)?;
        }

        for instr in self.instructions.iter() {
            instr.write_bytes(binary)?;
        }

        for byt in self.binary_data.iter() {
            binary.write_u8(*byt)?;
        }

        Ok(())
    }
}

/**
 * Serialized ObjectFormatHeader would look like (exclusive):
 * 0 - 8:   Magic
 * 8 - 16: length of sections
 * 16 - 20: version number
 */

pub const HEADER_SIZE: u64 = 8 * 2 + 4;

#[derive(Debug, Clone)]
struct ObjectFormatHeader {
    magic: u64,
    sections_length: u64, // sections count
    version: u32,
}

impl ObjectFormatHeader {
    fn new() -> Self {
        Self {
            magic: MAGIC_FORMAT_NUMBER,
            sections_length: 0,
            version: CURRENT_FORMAT_VERSION
        }
    }
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = ObjectFormatHeader::new();

        me.magic = binary.read_u64::<LittleEndian>()?;

        if me.magic != MAGIC_FORMAT_NUMBER {
            return Err(Error::new(io::ErrorKind::InvalidData, 
                format!("Invalid magic number! Invalid format specified!")));
        }

        me.sections_length = binary.read_u64::<LittleEndian>()?;
        me.version = binary.read_u32::<LittleEndian>()?;

        Ok(me)
    }
    fn write_bytes(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        binary.write_u64::<LittleEndian>(self.magic)?;
        binary.write_u64::<LittleEndian>(self.sections_length)?;
        binary.write_u32::<LittleEndian>(self.version)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Define {
    node: ParserNode
}

/**
 * Binary format description:
 * # HEADER
 * # SECTIONS
 * 
 * A tightly packed data structure
 */

#[derive(Debug, Clone)]
pub struct ObjectFormat {
    header: ObjectFormatHeader,
    defines: HashMap<String, Define>,
    pub sections: HashMap<String, SectionData>,
    compiler_instructions: HashMap<String, fn(&mut Self, &Vec<ParserNode>) -> Result<(), String>>,
    current_section: String
}

const DEFAULT_SECTION_NAME: &str = "text";

impl ObjectFormat {
    fn evaluate_expression(&self, _expr: &ParserNode) -> Result<ParserNode, String> {
        todo!()
    }

    // Compiler instructions
    fn _section_ci(&mut self, children: &Vec<ParserNode>) -> Result<(), String> {
        let child = match children.get(0) {
            Some(n) => n,
            None => {
                return Err(format!("Expected argument for 'section'"))
            }
        };
        match &child.node_type {
            NodeType::String(name) => {
                let mut sec = SectionData::new();
                sec.name = name.clone();

                self.current_section = sec.name.clone();

                if !self.sections.contains_key(&sec.name) {
                    self.sections.insert(sec.name.clone(), sec);
                    self.header.sections_length += 1;
                }

                Ok(())
            }
            _ => wrong_argument!(child, NodeType::String("".to_string()))
        }
    }
    fn _define_ci(&mut self, children: &Vec<ParserNode>) -> Result<(), String> {
        let name_node = match children.get(0) {
            Some(n) => n,
            None => {
                return Err(format!("Expected argument 0 for 'define'"))
            }
        };
        let data = match children.get(1) {
            Some(n) => n,
            None => {
                return Err(format!("Expected argument 1 for 'define'"))
            }
        };
        let name = match &name_node.node_type {
            NodeType::Identifier(name) => name,
            _ => wrong_argument!(name_node, NodeType::String(String::new()))
        };
        match &data.node_type {
            NodeType::Expression => {
                let n = self.evaluate_expression(data)?;
                self.defines.insert(name.clone(), Define {
                    node: n
                });
            }
            _ => {
                self.defines.insert(name.clone(), Define { node: data.clone() });
            }
        }
        Ok(())
    }
    fn _db_ci(&mut self, children: &Vec<ParserNode>) -> Result<(), String> {
        let sec = match self.sections.get_mut(&self.current_section) {
            Some(s) => s,
            None => {
                return Err(format!("Section '{}' not found! Maybe compiler bug?", self.current_section))
            }
        };

        if sec.instructions.len() != 0 {
            return Err(format!("Trying to add binary into section with instructions!"))
        }

        if children.len() == 0 {
            return Err(format!("Arguments expected for compiler instruction 'db'"))
        }

        sec.binary_section = true;

        for child in children {
            match &child.node_type {
                NodeType::ConstInteger(num) => {
                    if *num < 256 {
                        // im sorry, but i dont think this will throw an error
                        sec.binary_data.write_i8(*num as i8).unwrap();
                    } else if *num < 65536 {
                        sec.binary_data.write_i16::<LittleEndian>(*num as i16).unwrap();
                    } else {
                        sec.binary_data.write_i32::<LittleEndian>(*num as i32).unwrap();
                    }
                }
                NodeType::Negate | NodeType::Expression => {
                    todo!()
                }
                NodeType::String(some_str) => {
                    for b in some_str.bytes() {
                        sec.binary_data.write_u8(b).unwrap();
                    }
                }
                _ => unexpected_node!(child)
            }
        }

        Ok(())
    }
    fn _resb_ci(&mut self, children: &Vec<ParserNode>) -> Result<(), String> {
        let sec = match self.sections.get_mut(&self.current_section) {
            Some(s) => s,
            None => {
                return Err(format!("Section '{}' not found! Maybe compiler bug?", self.current_section))
            }
        };

        if sec.instructions.len() != 0 {
            return Err(format!("Trying to add binary into section with instructions!"))
        }

        sec.binary_section = true;

        let mut binary = Vec::<u8>::new();

        let child_node = match children.get(0) { 
            Some(c) => c,
            None => unexpected_eof!("RESB instruction requires 1 argument, 0 provided")
        };

        if let NodeType::ConstInteger(n) = child_node.node_type {
            for _ in 0..n {
                binary.push(0);
            }
        }

        sec.binary_data.append(&mut binary);

        Ok(())
    }
    // End compiler instructions

    pub fn new() -> Self {
        let mut me = Self {
            header: ObjectFormatHeader::new(),
            defines: HashMap::new(),
            sections: HashMap::new(),
            compiler_instructions: HashMap::new(),
            current_section: DEFAULT_SECTION_NAME.to_string(),
        };

        let default_section = SectionData::new();

        me.sections.insert(default_section.name.clone(), default_section);

        me.header.sections_length = 1;

        me.compiler_instructions.insert("section".to_string(), ObjectFormat::_section_ci);
        me.compiler_instructions.insert("define".to_string(), ObjectFormat::_define_ci);
        me.compiler_instructions.insert("db".to_string(), ObjectFormat::_db_ci);
        me.compiler_instructions.insert("resb".to_string(), ObjectFormat::_resb_ci);

        me
    }

    fn generate_binary(&self) -> Result<Vec<u8>, String> {
        let mut binary = Vec::<u8>::new();

        match self.header.write_bytes(&mut binary) {
            Ok(_) => {},
            Err(e) => {
                return Err(format!("Error occured while generating binary header: {}", e))
            }
        }

        for (sec_name, sec) in self.sections.iter() {
            match sec.write_bytes(&mut binary) {
                Ok(_) => {},
                Err(e) => {
                    return Err(format!("Error occured while generating \
                    binary for section '{}': {}", sec_name, e))
                }
            }
        }

        Ok(binary)
    }

    pub fn save_object(&self, path: &str) -> Result<(), String> {
        let binary = self.generate_binary()?;

        let mut file = match fs::File::create(path) {
            Ok(f) => f,
            Err(e) => {
                return Err(format!("Failed to open file to write: {e}"))
            }
        };
        
        match file.write_all(binary.as_slice()) {
            Ok(_) => (),
            Err(e) =>
                return Err(format!("Failed to write binary to file: {}", e))
        }

        Ok(())
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let mut me = Self::new();

        let mut binary_slice = bytes.as_slice();

        let header_parse_result = 
            ObjectFormatHeader::from_bytes(&mut binary_slice);
        
        me.header = match header_parse_result {
            Ok(header) => header,
            Err(e) => {
                return Err(format!("Error occured while parsing object file: {}", e))
            }
        };

        if me.header.version != CURRENT_FORMAT_VERSION {
            println!("Warning: File version does not match with latest format \
version! It may not be compatible!");
        }

        for _ in 0..me.header.sections_length {
            let section =
            match SectionData::from_bytes(&mut binary_slice) {
                Ok(section) => section,
                Err(e) => {
                    return Err(format!("Error occured while parsing section: {}", e))
                }
            };
            me.sections.insert(section.name.clone(), section);
        }

        Ok(me)
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = match fs::read(path) {
            Ok(vc) => vc,
            Err(e) => {
                return Err(format!("Error occured while reading file:\n{}", e))
            }
        };
        
        ObjectFormat::from_bytes(content)
    }

    fn do_compiler_instruction(&mut self, name: &str, children: &Vec<ParserNode>) -> Result<(), String> {
        let instr = match self.compiler_instructions.get(name) {
            Some(i) => i,
            None => bad_compinstr!(name)
        };
        instr(self, children)
    }

    fn process_instruction(&mut self, name: &str, children: &Vec<ParserNode>, current_label: &str) -> Result<(), String> {
        let registers = Registers::new();
        let conditions = Conditions::new();

        let instructions = Instructions::new();
        let opcode = match instructions.get_opcode(name) {
            Some(opc) => opc,
            None => {
                return Err(format!("Invalid instruction '{}'!", name))
            }
        };
        let instruction = instructions.get_instruction(opcode).unwrap();

        if instruction.args.len() != children.len() {
            return Err(format!("Argument count for instruction '{}' ({}) is incorrect! {} expected!",
            name, children.len(), instruction.args.len()))
        }

        let mut instr = InstructionData {
            opcode,
            references: Vec::new(),
            constants: Vec::new()
        };

        // XXX: Maybe refactor at some point
        // Idk when

        // Welcome to the hellhole
        // This is a stupid piece of code
        // And yes, I don't want to change it
        // Because it's perfect
        // There is nothing closer to perfection than this
        // You will understand it soon too
        // When you dive in this code
        // When you try to revise it
        // You will be able to see
        // How actually beautiful this code is
        // How accurate every character has been placed
        // How thin is the line between its life and death
        // And how easy it is to break it
        // Now, that you're warned
        // Go ahead. Do what you want
        // You don't need to bother yourself with this text anymore

        for i in 0..children.len() {
            let arg = &children[i];
            let expected_argument = instruction.args[i];
            match &arg.node_type { // TODO: Implement expressions
                NodeType::Identifier(identifier_name) => {
                    if self.defines.contains_key(identifier_name) {
                        let define_symbol = &self.defines[identifier_name];

                        match expected_argument {
                            ArgumentTypes::FloatingPoint |
                            ArgumentTypes::AbsPointer |
                            ArgumentTypes::RelPointer |
                            ArgumentTypes::Immediate32 => {
                                match &define_symbol.node.node_type {
                                    NodeType::ConstInteger(n) => {
                                        instr.constants.push(Constant { 
                                            argument_pos: i as u8, 
                                            size: ConstantSize::DoubleWord, 
                                            value: *n
                                        });
                                    }
                                    NodeType::ConstFloat(n) => {
                                        instr.constants.push(Constant { 
                                            argument_pos: i as u8,
                                            size: ConstantSize::DoubleWord,
                                            value: (*n).to_bits() as i64
                                        });
                                    }
                                    _ => unexpected_node!(define_symbol.node)
                                }
                            }
                            ArgumentTypes::Immediate16 => {
                                match &define_symbol.node.node_type {
                                    NodeType::ConstInteger(n) => {
                                        instr.constants.push(Constant { 
                                            argument_pos: i as u8, 
                                            size: ConstantSize::Word,
                                            value: *n & 0xFFFF
                                        });
                                    }
                                    _ => unexpected_node!(define_symbol.node)
                                }
                            }
                            ArgumentTypes::Immediate8 => {
                                match &define_symbol.node.node_type {
                                    NodeType::ConstInteger(n) => {
                                        instr.constants.push(Constant { 
                                            argument_pos: i as u8, 
                                            size: ConstantSize::Byte, 
                                            value: *n & 0xFF
                                        });
                                    }
                                    _ => unexpected_node!(define_symbol.node)
                                }
                            }
                            _ => unexpected_node!(define_symbol.node)
                        }
                    } else {
                        match expected_argument {
                            ArgumentTypes::Condition => {
                                let cond = match conditions.get_condition(identifier_name) {
                                    Some(c) => {c},
                                    None => unexpected_node!(arg)
                                };
                                instr.constants.push(Constant {
                                    argument_pos: i as u8,
                                    size: ConstantSize::Byte,
                                    value: *cond as i64
                                });
                            }
                            _ => {
                                let mut identifier = identifier_name.clone();
                                if identifier.starts_with('@') {
                                    identifier = current_label.to_string() + &identifier;
                                }
                                instr.references.push(Reference {
                                    argument_pos: i as u8,
                                    rf: identifier
                                })
                            }
                        }
                    }
                }
                NodeType::ConstFloat(n) => {
                    match expected_argument {
                        ArgumentTypes::FloatingPoint |
                        ArgumentTypes::Immediate32 => {
                            instr.constants.push(Constant {
                                argument_pos: i as u8,
                                size: ConstantSize::DoubleWord,
                                value: (*n).to_bits() as i64
                            });
                        }
                        _ => unexpected_node!(arg)
                    }
                }
                NodeType::ConstInteger(n) => {
                    match expected_argument {
                        ArgumentTypes::AbsPointer |
                        ArgumentTypes::RelPointer |
                        ArgumentTypes::Immediate32 => {
                            instr.constants.push(Constant {
                                argument_pos: i as u8,
                                size: ConstantSize::DoubleWord,
                                value: *n as i64
                            });
                        }
                        ArgumentTypes::Immediate16 => {
                            instr.constants.push(Constant {
                                argument_pos: i as u8,
                                size: ConstantSize::Word,
                                value: (*n & 0xFFFF) as i64
                            });
                        }
                        ArgumentTypes::Immediate8 => {
                            instr.constants.push(Constant {
                                argument_pos: i as u8,
                                size: ConstantSize::Byte,
                                value: (*n & 0xFF) as i64
                            });
                        }
                        _ => unexpected_node!(arg)
                    }
                }
                NodeType::Register(name) => {
                    match expected_argument {
                        ArgumentTypes::Register16 |
                        ArgumentTypes::Register32 |
                        ArgumentTypes::Register8 => {
                            instr.constants.push(Constant {
                                argument_pos: i as u8,
                                size: ConstantSize::Byte,
                                value: match registers.get(name) {
                                    Some(r) => *r as i64,
                                    None => {
                                        return Err(format!("Invalid register \
                                        name '{}'. Maybe compiler error!",
                                        name))
                                    }
                                }
                            });
                        }
                        _ => unexpected_node!(arg)
                    }
                }
                _ => unexpected_node!(arg)
            }
        }

        match self.sections.get_mut(&self.current_section) {
            Some(s) => s,
            None => {
                return Err(format!("Section '{}' does not exist! Maybe compiler bug?", self.current_section))
            }
        }.instructions.push(instr);
        
        Ok(())
    }

    pub fn load_parser_node(&mut self, node: &ParserNode) -> Result<(), String> {
        let instructions = Instructions::new();

        if node.node_type != NodeType::Program {
            return Err(format!("Cannot load not Program node into objgen"))
        }

        let mut current_label = String::new();

        for child in node.children.iter() {
            match &child.node_type {
                NodeType::CompilerInstruction(instr) => {
                    match self.do_compiler_instruction(instr, &child.children) {
                        Ok(_) => {},
                        Err(e) => {
                            return Err(format!("Error while executing compiler instruction: {}", e))
                        }
                    }
                }
                NodeType::Instruction(instr) => {
                    match self.process_instruction(instr, &child.children, &current_label) {
                        Ok(_) => {},
                        Err(e) => {
                            return Err(format!("Error while processing instruction: {}", e))
                        }
                    }
                }
                NodeType::Label(name) => {
                    let current_section = match self.sections.get_mut(&self.current_section) {
                        Some(s) => s,
                        None => {
                            return Err(format!("Section '{}' does not exist! Maybe compiler bug?", self.current_section))
                        }
                    };
                    let mut binlen = 0usize;

                    if current_section.binary_data.len() == 0 {
                        for instrs in current_section.instructions.iter() {
                            binlen += instructions.get_instruction(instrs.opcode).unwrap().get_size();
                        }
                    } else {
                        binlen = current_section.binary_data.len();
                    }

                    if current_section.labels.contains_key(name) {
                        return Err(format!("Label '{}' is redefined!", name))
                    }

                    let label = ObjectLabelSymbol {
                        name: name.clone(),
                        ptr_instr: current_section.instructions.len() as u64,
                        ptr_binary: binlen as u64,
                    };
                    
                    current_section.labels.insert(name.clone(), label);
                    current_label = name.clone();
                }
                _ => unexpected_node!(child)
            }
        }

        Ok(())
    }
}
