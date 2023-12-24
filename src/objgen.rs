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
const CURRENT_FORMAT_VERSION: u32 = 4;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq)]
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
    pub fn get_args(&self) -> String {
        let instructions = Instructions::new();
        let registers = Registers::new();

        // FIXME: Unwrap, maybe?
        let sym = instructions.get_instruction(self.opcode).unwrap();

        let mut result = String::new();
        
        let mut consts = self.constants.iter();
        let mut refs = self.references.iter();

        let argc = consts.len() + refs.len();

        for i in 0..argc {
            match refs.find(|r| r.argument_pos == (i as u8)) {
                Some(r) => {
                    result += &format!("{} ", r.rf);
                    continue
                },
                None => {}
            }
            match consts.find(|c| c.argument_pos == (i as u8)) {
                Some(c) => {
                    match sym.args[i] {
                        ArgumentTypes::Register16 => {
                            let name = match registers.get_name16(c.value as u8) {
                                Some(s) => s,
                                None => "(UREG)"
                            };
                            result += &format!("{} ", name);
                        }
                        ArgumentTypes::Register32 => {
                            let name = match registers.get_name32(c.value as u8) {
                                Some(s) => s,
                                None => "(UREG)"
                            };
                            result += &format!("{} ", name);
                        }
                        ArgumentTypes::Register8 => {
                            let name = match registers.get_name8(c.value as u8) {
                                Some(s) => s,
                                None => "(UREG)"
                            };
                            result += &format!("{} ", name);
                        }
                        _ => {
                            result += &format!("{:#04x} ({:?}) ", c.value, c.size);
                        }
                    }
                    continue
                }
                None => {}
            }
        }

        result
    }
}

/**
 * 0 - 8: ptr
 * 8 - <>: name
 */
#[derive(Debug, Clone)]
pub struct ObjectLabelSymbol {
    name: String,
    pub ptr: u64,
}

impl ObjectLabelSymbol {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            name: String::new(),
            ptr: 0,
        };

        me.ptr = binary.read_u64::<LittleEndian>()?;

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
        binary.write_u64::<LittleEndian>(self.ptr)?;

        for b in self.name.bytes() {
            binary.write_u8(b)?;
        }
        binary.write_u8(0)?;

        Ok(())
    }
}

/**
 * Binary reference structure:
 * 0 - 1: size
 * 1 - <>: name
 */
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryReference {
    pub rf: String,
    pub size: ConstantSize
}

impl BinaryReference {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let size = match ConstantSize::from_u8(binary.read_u8()?) {
            Some(s) => s,
            None => {
                return Err(Error::new(io::ErrorKind::InvalidData,
                format!("Error occured loading BinaryConstant: invalid size")))
            }
        };

        let mut char_vec = Vec::<u8>::new();

        let mut c = binary.read_u8()?;

        while c != 0 {
            char_vec.push(c);
            c = binary.read_u8()?;
        }

        Ok(Self {
            size,
            rf: String::from_utf8(char_vec).unwrap()
        })
    }
    fn write_bytes(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        binary.write_u8(self.size.to_u8())?;

        for b in self.rf.bytes() {
            binary.write_u8(b)?;
        }
        binary.write_u8(0)?;

        Ok(())
    }
}

/**
 * Binary const structure:
 * 0 - 1: size
 * 1 - 9: value
 */
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryConstant {
    pub size: ConstantSize,
    pub value: i64
}

impl BinaryConstant {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let size = binary.read_u8()?;
        let value = binary.read_i64::<LittleEndian>()?;

        Ok(Self {
            size: match ConstantSize::from_u8(size) {
                Some(s) => s,
                None => {
                    return Err(Error::new(io::ErrorKind::InvalidData,
                    format!("Error occured loading BinaryConstant: invalid size")))
                }
            },
            value
        })
    }
    fn write_binary(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        binary.write_u8(self.size.to_u8())?;
        binary.write_i64::<LittleEndian>(self.value)?;

        Ok(())
    }
}

/**
 * Binary unit structure description
 * 0 - 1: Type (0 is const, 1 is ref)
 * <data>
 */
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryUnit {
    pub reference: Option<BinaryReference>,
    pub constant: Option<BinaryConstant>
}

impl BinaryUnit {
    pub fn get_size(&self) -> Option<usize> {
        if let Some(cst) = &self.constant {
            Some(cst.size.get_size())
        } else if let Some(reference) = &self.reference {
            Some(reference.size.get_size())
        } else {
            None
        }
    }
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            reference: None,
            constant: None
        };
        
        let typ = binary.read_u8()?;

        match typ {
            0 => {
                me.constant = Some(BinaryConstant::from_bytes(binary)?)
            },
            1 => {
                me.reference = Some(BinaryReference::from_bytes(binary)?)
            },
            _ => {
                return Err(Error::new(io::ErrorKind::InvalidData, 
                    format!("Invalid type for binary unit. Bad format specified.")))
            }
        }

        Ok(me)
    }
    fn write_bytes(&self, binary: &mut Vec<u8>) -> Result<(), Error> {
        if let Some(cst) = &self.constant {
            binary.write_u8(0)?;
            cst.write_binary(binary)?;
        } else if let Some(reference) = &self.reference {
            binary.write_u8(1)?;
            reference.write_bytes(binary)?;
        } else {
            return Err(Error::new(io::ErrorKind::InvalidData, 
                format!("BinaryUnit without information!")))
        }
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
//    pub binary_data: Vec<u8>,
    pub binary_data: Vec<BinaryUnit>,
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
                label.ptr += old_bin_length;
                self.labels.insert(label_name, label);
            }
        } else {
            let old_instr_length = self.instructions.len() as u64;
            self.instructions.append(&mut other.instructions);
            
            for (label_name, mut label) in other.labels {
                if self.labels.contains_key(&label_name) {
                    return Err(format!("Cannot merge two binary sections with similar labels!"))
                }
                label.ptr += old_instr_length;
                self.labels.insert(label_name, label);
            }
        }

        Ok(())
    }

    pub fn get_binary_size(&self) -> usize {
        if self.binary_section {
            let mut binary_len = 0;

            for unit in self.binary_data.iter() {
                // unwrap because we assume this is valid from object file
                binary_len += unit.get_size().unwrap();
            }

            return binary_len
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
            let mut binary_index = 0;

            for (i, unit) in self.binary_data.iter().enumerate() {
                if i as u64 == index { break }
                // unwrap because we assume this is valid from object file
                binary_index += unit.get_size().unwrap();
            }

            return binary_index as u64
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

        if self.binary_section { return Some(label.ptr) }

        Some(self.get_binary_position(label.ptr))
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
            let bin = BinaryUnit::from_bytes(binary)?;
            me.binary_data.push(bin);
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
            byt.write_bytes(binary)?;
            //binary.write_u8(*byt)?;
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
pub struct ObjectFormatHeader {
    magic: u64,
    pub sections_length: u64, // sections count
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
    pub header: ObjectFormatHeader,
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
                NodeType::Identifier(sym_name) => {
                    sec.binary_data.push(BinaryUnit {
                        constant: None,
                        reference: Some(BinaryReference {
                            size: ConstantSize::Byte,
                            rf: sym_name.clone()
                        })
                    });
                }
                NodeType::ConstInteger(num) => {
                    if *num < 256 {
                        sec.binary_data.push(BinaryUnit {
                            constant: Some(BinaryConstant {
                                size: ConstantSize::Byte,
                                value: *num
                            }),
                            reference: None
                        });
                    } else if *num < 65536 {
                        sec.binary_data.push(BinaryUnit {
                            constant: Some(BinaryConstant {
                                size: ConstantSize::Word,
                                value: *num
                            }),
                            reference: None
                        });
                    } else {
                        sec.binary_data.push(BinaryUnit {
                            constant: Some(BinaryConstant {
                                size: ConstantSize::DoubleWord,
                                value: *num
                            }),
                            reference: None
                        });
                    }
                }
                NodeType::Negate | NodeType::Expression => {
                    todo!()
                }
                NodeType::String(some_str) => {
                    for b in some_str.bytes() {
                        sec.binary_data.push(BinaryUnit {
                            constant: Some(BinaryConstant {
                                size: ConstantSize::Byte,
                                value: b as i64
                            }),
                            reference: None
                        });
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

        let mut binary = Vec::<BinaryUnit>::new();

        let child_node = match children.get(0) { 
            Some(c) => c,
            None => unexpected_eof!("RESB instruction requires 1 argument, 0 provided")
        };

        if let NodeType::ConstInteger(n) = child_node.node_type {
            for _ in 0..n {
                binary.push(BinaryUnit {
                    reference: None,
                    constant: Some(BinaryConstant {
                        size: ConstantSize::Byte,
                        value: 0
                    })
                });
            }
        }

        sec.binary_data.append(&mut binary);

        Ok(())
    }
    // Reads binary data from file and inserts it as binary data into section
    fn _data_ci(&mut self, children: &Vec<ParserNode>) -> Result<(), String> {
        let sec = match self.sections.get_mut(&self.current_section) {
            Some(s) => s,
            None => {
                return Err(format!("Section '{}' not found! Maybe compiler bug?", self.current_section))
            }
        };

        if !sec.binary_section || sec.instructions.len() != 0 {
            return Err(format!("Trying to add binary into section with instructions!"))
        }

        let child_node = match children.get(0) { 
            Some(c) => c,
            None => unexpected_eof!("DATA instruction requires 1 argument, 0 provided")
        };

        if let NodeType::String(path) = &child_node.node_type {
            let data = match fs::read(path) {
                Ok(d) => d,
                Err(e) => {
                    return Err(format!("Error occured while reading file: {e}"))
                }
            };
            for b in data {
                sec.binary_data.push(BinaryUnit {
                    reference: None,
                    constant: Some(BinaryConstant {
                        size: ConstantSize::Byte,
                        value: b as i64
                    })
                })
            }
        } else {
            return Err(format!("DATA instruction takes String. {:?} provided", child_node.node_type))
        }

        Ok(())
    }
    // Define double word, same as db but for dw
    fn _dd_ci(&mut self, children: &Vec<ParserNode>) -> Result<(), String> {
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
                NodeType::Identifier(sym_name) => {
                    sec.binary_data.push(BinaryUnit {
                        constant: None,
                        reference: Some(BinaryReference {
                            size: ConstantSize::DoubleWord,
                            rf: sym_name.clone()
                        })
                    });
                }
                NodeType::ConstInteger(num) => {
                    sec.binary_data.push(BinaryUnit {
                        reference: None,
                        constant: Some(BinaryConstant {
                            size: ConstantSize::DoubleWord,
                            value: *num
                        })
                    });
                }
                NodeType::Negate | NodeType::Expression => {
                    todo!()
                }
                NodeType::String(some_str) => {
                    for b in some_str.bytes() {
                        sec.binary_data.push(BinaryUnit {
                            reference: None,
                            constant: Some(BinaryConstant {
                                size: ConstantSize::DoubleWord,
                                value: b as i64
                            })
                        });
                    }
                }
                _ => unexpected_node!(child)
            }
        }

        Ok(())
    }
    // Define word, same as db but for w
    fn _dw_ci(&mut self, children: &Vec<ParserNode>) -> Result<(), String> {
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
                NodeType::Identifier(sym_name) => {
                    sec.binary_data.push(BinaryUnit {
                        constant: None,
                        reference: Some(BinaryReference {
                            size: ConstantSize::Word,
                            rf: sym_name.clone()
                        })
                    });
                }
                NodeType::ConstInteger(num) => {
                    sec.binary_data.push(BinaryUnit {
                        reference: None,
                        constant: Some(BinaryConstant {
                            size: ConstantSize::Word,
                            value: *num
                        })
                    });
                }
                NodeType::Negate | NodeType::Expression => {
                    todo!()
                }
                NodeType::String(some_str) => {
                    for b in some_str.bytes() {
                        sec.binary_data.push(BinaryUnit {
                            reference: None,
                            constant: Some(BinaryConstant {
                                size: ConstantSize::Word,
                                value: b as i64
                            })
                        });
                    }
                }
                _ => unexpected_node!(child)
            }
        }

        Ok(())
    }
    // End compiler instructions

    pub fn create_jumper(entrypoint: String) -> Self {
        let mut me = Self::new();

        let mut section = SectionData::new();
        section.instructions.push(InstructionData {
            opcode: 12, // jpr opcode
            references: vec![Reference {
                argument_pos: 0,
                rf: entrypoint
            }],
            constants: Vec::new()
        });
        me.sections.insert(section.name.clone(), section);

        me
    }

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
        me.compiler_instructions.insert("data".to_string(), ObjectFormat::_data_ci);
        me.compiler_instructions.insert("dd".to_string(), ObjectFormat::_dd_ci);
        me.compiler_instructions.insert("dw".to_string(), ObjectFormat::_dw_ci);

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

    fn resolve_define(&self, arg: usize, instr: &mut InstructionData, expected_argument: &ArgumentTypes, define_symbol: &Define, depth: i32)
        -> Result<(), String>
    {
        if let NodeType::Identifier(iden) = &define_symbol.node.node_type {
            if depth > 100 {
                return Err(format!("Looping defines detected!"))
            }
            if self.defines.contains_key(iden) {
                self.resolve_define(
                    arg,
                    instr,
                    expected_argument,
                    &self.defines[iden],
                    depth + 1
                )?;
            }
            return Ok(())
        }
        match expected_argument {
            ArgumentTypes::FloatingPoint |
            ArgumentTypes::AbsPointer |
            ArgumentTypes::RelPointer |
            ArgumentTypes::Immediate32 => {
                match &define_symbol.node.node_type {
                    NodeType::ConstInteger(n) => {
                        instr.constants.push(Constant { 
                            argument_pos: arg as u8, 
                            size: ConstantSize::DoubleWord, 
                            value: *n
                        });
                    }
                    NodeType::ConstFloat(n) => {
                        instr.constants.push(Constant { 
                            argument_pos: arg as u8,
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
                            argument_pos: arg as u8, 
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
                            argument_pos: arg as u8, 
                            size: ConstantSize::Byte, 
                            value: *n & 0xFF
                        });
                    }
                    _ => unexpected_node!(define_symbol.node)
                }
            }
            _ => unexpected_node!(define_symbol.node)
        }
        Ok(())
    }

    fn resolve_instruction(&self, 
        arg: &ParserNode, 
        instr: &mut InstructionData,
        expected_argument: &ArgumentTypes,
        index: usize,
        current_label: &str
    ) -> Result<(), String>
    {
        let conditions = Conditions::new();
        let registers = Registers::new();

        match &arg.node_type { // TODO: Implement expressions
            NodeType::Identifier(identifier_name) => {
                if self.defines.contains_key(identifier_name) {
                    let define_symbol = &self.defines[identifier_name];

                    self.resolve_define(index, instr, &expected_argument, define_symbol, 0)?;
                } else {
                    match expected_argument {
                        ArgumentTypes::Condition => {
                            let cond = match conditions.get_condition(identifier_name) {
                                Some(c) => {c},
                                None => unexpected_node!(arg)
                            };
                            instr.constants.push(Constant {
                                argument_pos: index as u8,
                                size: ConstantSize::Byte,
                                value: *cond as i64
                            });
                        }
                        _ => {
                            let mut identifier = identifier_name.clone();
                            if identifier.starts_with('@') {
                                identifier = current_label.to_string() + &identifier;
                            } else if identifier == "@" {
                                identifier = current_label.to_string();
                            }
                            instr.references.push(Reference {
                                argument_pos: index as u8,
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
                            argument_pos: index as u8,
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
                            argument_pos: index as u8,
                            size: ConstantSize::DoubleWord,
                            value: *n as i64
                        });
                    }
                    ArgumentTypes::Immediate16 => {
                        instr.constants.push(Constant {
                            argument_pos: index as u8,
                            size: ConstantSize::Word,
                            value: (*n & 0xFFFF) as i64
                        });
                    }
                    ArgumentTypes::Immediate8 => {
                        instr.constants.push(Constant {
                            argument_pos: index as u8,
                            size: ConstantSize::Byte,
                            value: (*n & 0xFF) as i64
                        });
                    }
                    _ => unexpected_node!(arg)
                }
            }
            NodeType::Register(name) => {
                match expected_argument {
                    ArgumentTypes::Register16 => {
                        instr.constants.push(Constant {
                            argument_pos: index as u8,
                            size: ConstantSize::Byte,
                            value: match registers.get16(name) {
                                Some(r) => *r as i64,
                                None => {
                                    return Err(format!("Invalid 16 bit register \
                                    name '{}'.", name))
                                }
                            }
                        });
                    }
                    ArgumentTypes::Register32 => {
                        instr.constants.push(Constant {
                            argument_pos: index as u8,
                            size: ConstantSize::Byte,
                            value: match registers.get32(name) {
                                Some(r) => *r as i64,
                                None => {
                                    return Err(format!("Invalid 32 bit register \
                                    name '{}'.", name))
                                }
                            }
                        });
                    }
                    ArgumentTypes::Register8 => {
                        instr.constants.push(Constant {
                            argument_pos: index as u8,
                            size: ConstantSize::Byte,
                            value: match registers.get8(name) {
                                Some(r) => *r as i64,
                                None => {
                                    return Err(format!("Invalid 8 bit register \
                                    name '{}'.", name))
                                }
                            }
                        });
                    }
                    _ => unexpected_node!(arg)
                }
            }
            _ => unexpected_node!(arg)
        }
        Ok(())
    }

    fn process_instruction(&mut self, name: &str, children: &Vec<ParserNode>, current_label: &str) -> Result<(), String> {
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

        for i in 0..children.len() {
            let arg = &children[i];
            let expected_argument = instruction.args[i];

            self.resolve_instruction(arg, &mut instr, &expected_argument, i, current_label)?;
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
        //let instructions = Instructions::new();

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
                    let pointer: usize;

                    if current_section.binary_data.len() == 0 {
                        pointer = current_section.instructions.len();
                    } else {
                        pointer = current_section.binary_data.len();
                    }

                    if current_section.labels.contains_key(name) {
                        return Err(format!("Label '{}' is redefined!", name))
                    }

                    let label = ObjectLabelSymbol {
                        name: name.clone(),
                        ptr: pointer as u64,
                    };
                    
                    current_section.labels.insert(name.clone(), label);
                    
                    if !name.contains('@') {
                        // FIXME: This is the easiest fix i can think about now
                        current_label = name.clone();
                    }
                }
                _ => unexpected_node!(child)
            }
        }

        Ok(())
    }
}
