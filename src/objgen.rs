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
const CURRENT_FORMAT_VERSION: u32 = 1;

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

        // TODO: Return SIZE INCORRECT error
        me.size = ConstantSize::from_u8(binary.read_u8()?).unwrap();

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
    ref_count: u8,
    const_count: u8,
    opcode: u16,
    references: Vec<Reference>,
    constants: Vec<Constant>
}

impl InstructionData {
    fn from_bytes(mut binary: &[u8]) -> Result<Self, Error> {
        let mut me = Self {
            ref_count: 0,
            const_count: 0,
            opcode: 0xFFFF,
            references: Vec::new(),
            constants: Vec::new()
        };

        me.opcode = binary.read_u16::<LittleEndian>()?;
        me.ref_count = binary.read_u8()?;
        me.const_count = binary.read_u8()?;

        let mut ptr = 4u16;

        for _ in 0..me.ref_count {
            let reference = Reference::from_bytes(&mut binary)?;
            me.references.push(reference);
        }

        for _ in 0..me.const_count {
            let constant = Constant::from_bytes(&mut binary)?;
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
 * 9 - infinity: name
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
 * Serialized ObjectFormatHeader would look like (exclusive):
 * 0 - 8:   Magic
 * 8 - 16:  offset to labelinfo
 * 16 - 24: offset to instructions
 * 24 - 32: offset to data
 * 32 - 40: offset to section_info
 * 40 - 48: length of labelinfo
 * 48 - 56: length of instructions
 * 56 - 64: length of data
 * 64 - 72: length of sections
 * 72 - 76: version number
 */

const HEADER_SIZE: u64 = 8 * 9 + 4;

#[derive(Debug)]
struct ObjectFormatHeader {
    magic: u64,
    labelinfo_length: u64, // label count
    instructions_length: u64, // instruction count
    data_length: u64, // data length (in bytes)
    sections_length: u64, // sections count
    version: u32,
}

impl ObjectFormatHeader {
    fn new() -> Self {
        Self {
            magic: MAGIC_FORMAT_NUMBER,
            labelinfo_length: 0,
            instructions_length: 0,
            sections_length: 0,
            data_length: 0,
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

        /*me.offset_to_labelinfo = binary.read_u64::<LittleEndian>()?;
        me.offset_to_instructions = binary.read_u64::<LittleEndian>()?;
        me.offset_to_data = binary.read_u64::<LittleEndian>()?;
        me.offset_to_sections = binary.read_u64::<LittleEndian>()?;*/
        me.labelinfo_length = binary.read_u64::<LittleEndian>()?;
        me.instructions_length = binary.read_u64::<LittleEndian>()?;
        me.data_length = binary.read_u64::<LittleEndian>()?;
        me.sections_length = binary.read_u64::<LittleEndian>()?;
        me.version = binary.read_u32::<LittleEndian>()?;

        Ok(me)
    }
}

/**
 * Binary format description:
 * # HEADER
 * # SECTIONS (NYI)
 * # LABELINFO
 * # INSTRUCTIONS
 * # DATA
 * 
 * A tightly packed data structure
 */

#[derive(Debug)]
pub struct ObjectFormat {
    header: ObjectFormatHeader,
    defines: HashMap<String, i64>, // Defines can be only numbers
    label_symbols: Vec<ObjectLabelSymbol>,
    instruction_symbols: Vec<InstructionData>,
    data_binary: Vec<u8>,
    current_section: String
}

impl ObjectFormat {
    pub fn new() -> Self {
        Self {
            header: ObjectFormatHeader::new(),
            defines: HashMap::new(),
            label_symbols: Vec::new(),
            current_section: "text".to_string(),
            instruction_symbols: Vec::new(),
            data_binary: Vec::new()
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let mut me = Self::new();

        let bytes_copy = bytes.clone();
        let mut binary_slice = bytes_copy.as_slice();

        let header_parse_result = 
            ObjectFormatHeader::from_bytes(&mut binary_slice);
        
        me.header = match header_parse_result {
            Ok(header) => header,
            Err(e) => {
                return Err(format!("Error occured while parsing object file: {}", e))
            }
        };

        for _ in 0..me.header.labelinfo_length {
            let label = 
            match ObjectLabelSymbol::from_bytes(&mut binary_slice) {
                Ok(label) => label,
                Err(e) => {
                    return Err(format!("Error occured while parsing label information from object: {}", e))
                }
            };
            me.label_symbols.push(label);
        }
        for _ in 0..me.header.instructions_length {
            let instruction = 
            match InstructionData::from_bytes(&mut binary_slice) {
                Ok(label) => label,
                Err(e) => {
                    return Err(format!("Error occured while parsing instructions from object: {}", e))
                }
            };
            me.instruction_symbols.push(instruction);
        }

        for _ in 0..me.header.data_length {
            let byte = match binary_slice.read_u8() {
                Ok(n) => n,
                Err(e) => {
                    return Err(
                        format!("Error occured while parsing a number: {}", e)
                    )
                }
            };
            me.data_binary.push(byte);
        }

        println!("{:?}", me);

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
