use std::collections::HashMap;
use std::io::Error;
use std::mem::size_of;
use std::str::Utf8Error;
use std::{fs, io, str};
use byteorder::{LittleEndian, ReadBytesExt};

use crate::parser::{ParserNode, NodeType, Registers};
use crate::symbols::{Instructions, ArgumentTypes};

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
macro_rules! argument_eof {
    () => {
        return Err(format!("Unexpected end of arguments"))
    };
}

const MAGIC_FORMAT_NUMBER: u64 = 0x3A6863FC6173371B;
const CURRENT_FORMAT_VERSION: u32 = 2;

/**
 * 0 - 1: argument position
 * 1 - <>: reference name
 */
#[derive(Debug)]
struct Reference {
    argument_pos: u8,
    rf: String
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
}
#[derive(Debug)]
enum ConstantSize {
    Byte, Word, DoubleWord
}

impl ConstantSize {
    fn from_u8(n: u8) -> Option<Self> {
        match n {
            1 => Some(ConstantSize::Byte),
            2 => Some(ConstantSize::Word),
            4 => Some(ConstantSize::DoubleWord),
            _ => None
        }
    }
}

/**
 * 0 - 1: argument position
 * 1 - 2: const size
 * 2 - 10: value
 */
#[derive(Debug)]
struct Constant {
    argument_pos: u8,
    size: ConstantSize,
    value: u64
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
                return Err(Error::new(io::ErrorKind::UnexpectedEof,
                format!("Wrong constant size in instruction!")))
            }
        };

        me.value = match me.size {
            ConstantSize::Byte => binary.read_u8()? as u64,
            ConstantSize::Word => binary.read_u16::<LittleEndian>()? as u64,
            ConstantSize::DoubleWord => binary.read_u32::<LittleEndian>()? as u64,
        };

        Ok(me)
    }
}

/**
 * 0 - 2: opcode
 * 2 - 3: reference count
 * 3 - 4: constant count
 * 4 - <>: references
 * <> - <>: constants
 */

#[derive(Debug)]
struct InstructionData {
    opcode: u16,
    references: Vec<Reference>,
    constants: Vec<Constant>
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
}

/**
 * 0 - 8: ptr
 * 8 - 9: ptr_to_binary
 * 9 - <>: name
 */
#[derive(Debug)]
pub struct ObjectLabelSymbol {
    name: String,
    ptr: u64,
    ptr_to_binary: bool // ptr to binary or instructions
}

impl ObjectLabelSymbol {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            name: String::new(),
            ptr: 0,
            ptr_to_binary: false
        };

        me.ptr = binary.read_u64::<LittleEndian>()?;
        me.ptr_to_binary = binary.read_u8()? != 0;

        let mut char_vec = Vec::<u8>::new();

        let mut c = binary.read_u8()?;

        while c != 0 {
            char_vec.push(c);
            c = binary.read_u8()?;
        }

        me.name = String::from_utf8(char_vec).unwrap();

        Ok(me)
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
#[derive(Debug)]
struct SectionData {
    name: String,
    instructions: Vec<InstructionData>,
    labels: Vec<ObjectLabelSymbol>,
    binary_data: Vec<u8>
}

impl SectionData {
    fn from_bytes(binary: &mut &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            name: String::new(),
            instructions: Vec::new(),
            labels: Vec::new(),
            binary_data: Vec::new()
        };

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
            me.labels.push(label);
        }

        for _ in 0..instruction_count {
            let instruction = InstructionData::from_bytes(binary)?;
            me.instructions.push(instruction);
        }

        for _ in 0..binary_count {
            let binary = binary.read_u8()?;
            me.binary_data.push(binary);
        }

        Ok(me)
    }
}

/**
 * Serialized ObjectFormatHeader would look like (exclusive):
 * 0 - 8:   Magic
 * 8 - 16: length of sections
 * 16 - 20: version number
 */

const HEADER_SIZE: u64 = 8 * 2 + 4;

#[derive(Debug)]
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
}

/**
 * Binary format description:
 * # HEADER
 * # SECTIONS
 * 
 * A tightly packed data structure
 */

#[derive(Debug)]
pub struct ObjectFormat {
    header: ObjectFormatHeader,
    defines: HashMap<String, i64>, // Defines can be only numbers
    sections: Vec<SectionData>,
}

impl ObjectFormat {
    pub fn new() -> Self {
        Self {
            header: ObjectFormatHeader::new(),
            defines: HashMap::new(),
            sections: Vec::new()
        }
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
            me.sections.push(section);
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
    pub fn load_parser_node(&mut self, node: &ParserNode) -> Result<(), String> {
        todo!()
    }
}
