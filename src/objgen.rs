use std::collections::HashMap;
use std::mem::size_of;
use base64::prelude::*;
use std::{fs, io};

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

const MAGIC_FORMAT: [u8; 8] = [ 0x13, 0x37, 0x73, 0x61, 0x72, 0x63, 0x68, 0x34 ];
const MAGIC_FORMAT_NUMBER: u64 = 0x3468637261733713; // Exactly the same as above
const CURRENT_FORMAT_VERSION: u32 = 1;

enum ObjectInstructionArgument {
    None, Value, Reference
}

pub struct ObjectInstructionSymbol {
    opcode: u16,
    args: [ObjectInstructionArgument; 2]
}

/**
 * Serialized ObjectLabelSymbol would look like:
 * 0 - 32 - ASCII section name
 * 32 - 64 - ASCII label name
 * 64 - 72 - offset from binary start
 */
#[derive(Debug)]
pub struct ObjectLabelSymbol {
    section: [u8; 32],
    name: [u8; 32],
    offset: u64 // offset from binary start
}

/**
 * Serialized ObjectFormatHeader would look like:
 * 0 - 8:   Magic
 * 8 - 16:  offset to labelinfo
 * 16 - 24: offset to binary
 * 24 - 32: length of labelinfo
 * 32 - 40: length of binary
 * 40 - 44: version number
 */
#[derive(Debug)]
struct ObjectFormatHeader {
    magic: u64,
    offset_to_labelinfo: u64,
    offset_to_binary: u64,
    labelinfo_length: u64,
    binary_length: u64,
    version: u32,
}

#[derive(Debug)]
pub struct ObjectFormat {
    header: ObjectFormatHeader,
    defines: HashMap<String, i64>, // Defines can be only numbers
    pub symbols: Vec<ObjectLabelSymbol>,
    current_section: String
}

// Method from https://stackoverflow.com/questions/36669427/does-rust-have-a-way-to-convert-several-bytes-to-a-number
fn as_u32_be(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 24) +
    ((array[1] as u32) << 16) +
    ((array[2] as u32) <<  8) +
    ((array[3] as u32) <<  0)
}
fn as_u64_be(array: &[u8; 8]) -> u64 {
    ((array[0] as u64) << 56) +
    ((array[1] as u64) << 48) +
    ((array[2] as u64) << 40) +
    ((array[3] as u64) << 32) +
    ((array[4] as u64) << 24) +
    ((array[5] as u64) << 16) +
    ((array[6] as u64) <<  8) +
    ((array[7] as u64) <<  0)
}
fn as_u32_le(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 0) +
    ((array[1] as u32) << 8) +
    ((array[2] as u32) << 16) +
    ((array[3] as u32) << 24)
}
fn as_u64_le(array: &[u8; 8]) -> u64 {
    ((array[0] as u64) << 0) +
    ((array[1] as u64) << 8) +
    ((array[2] as u64) << 16) +
    ((array[3] as u64) << 24) +
    ((array[4] as u64) << 32) +
    ((array[5] as u64) << 40) +
    ((array[6] as u64) << 48) +
    ((array[7] as u64) << 56)
}

impl ObjectFormat {
    pub fn new() -> Self {
        Self {
            header: ObjectFormatHeader { 
                magic: MAGIC_FORMAT_NUMBER,
                offset_to_labelinfo: size_of::<ObjectFormatHeader>() as u64,
                offset_to_binary: size_of::<ObjectFormatHeader>() as u64,
                labelinfo_length: 0,
                binary_length: 0,
                version: CURRENT_FORMAT_VERSION
            },
            defines: HashMap::new(),
            symbols: Vec::new(),
            current_section: "text".to_string()
        }
    }

    fn parse_labels(offset: u64, length: u64, bytes: &Vec<u8>) {
        let off = offset as usize;
        let len = length as usize;

        let mut ptr = 0usize;

        while ptr < len {

        }
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let magic: &[u8; 8] = bytes.as_slice()[0..8].try_into().unwrap(); // FIXME: fix unwrap
        let magic_num = as_u64_le(magic);

        if magic_num != MAGIC_FORMAT_NUMBER {
            return Err(format!("Incorrect object format (magic number does not match)"))
        }

        let offset_lbl: &[u8; 8] = bytes.as_slice()[8..16].try_into().unwrap(); // FIXME: fix unwrap
        let offset_lbl_num: u64 = as_u64_le(offset_lbl);

        let offset_bin: &[u8; 8] = bytes.as_slice()[16..24].try_into().unwrap(); // FIXME: fix unwrap
        let offset_bin_num: u64 = as_u64_le(offset_bin);

        let len_lbl: &[u8; 8] = bytes.as_slice()[24..32].try_into().unwrap(); // FIXME: fix unwrap
        let len_lbl_num: u64 = as_u64_le(len_lbl);

        let len_bin: &[u8; 8] = bytes.as_slice()[32..40].try_into().unwrap(); // FIXME: fix unwrap
        let len_bin_num: u64 = as_u64_le(len_bin);

        let ver: &[u8; 4] = bytes.as_slice()[40..44].try_into().unwrap(); // FIXME: fix unwrap
        let ver_num: u32 = as_u32_le(ver);

        let header = ObjectFormatHeader {
            magic: magic_num,
            offset_to_labelinfo: offset_lbl_num,
            offset_to_binary: offset_bin_num,
            labelinfo_length: len_lbl_num,
            binary_length: len_bin_num,
            version: ver_num
        };
        let mut me = Self::new();

        me.header = header;

        

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

#[test]
fn test_cast() {
    let bytes: Vec<u8> = vec![
        0x13, 0x37, 0x73, 0x61, 0x72, 0x63, 0x68, 0x34, // magic
        0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // offset label
        0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // offset binary
        0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // len label
        0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // len binary
        0x01, 0x00, 0x00, 0x00, // ver
    ];
    let fmt = ObjectFormat::from_bytes(bytes).unwrap();
    println!("{:?}", fmt.header);

    assert_eq!(fmt.header.magic, MAGIC_FORMAT_NUMBER);
}
