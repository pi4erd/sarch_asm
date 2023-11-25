use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub enum ArgumentTypes {
    AbsPointer, RelPointer,
    Register32, Register16, Register8,
    Immediate32, Immediate16, Immediate8,
    FloatingPoint
}

#[derive(Clone, Debug)]
pub struct Instruction {
    pub name: &'static str,
    pub opcode: u16,
    pub args: Vec<ArgumentTypes>
}

impl Instruction {
    pub fn extended_opcode(&self) -> bool {
        self.opcode & 0x80 != 0
    }
    pub fn get_size(&self) -> usize {
        let mut size = if self.extended_opcode() { 2usize } else { 1usize };

        for arg in self.args.iter() {
            size += match arg {
                ArgumentTypes::AbsPointer |
                ArgumentTypes::RelPointer |
                ArgumentTypes::FloatingPoint |
                ArgumentTypes::Immediate32 => 4,

                ArgumentTypes::Register16 |
                ArgumentTypes::Register32 |
                ArgumentTypes::Register8 |
                ArgumentTypes::Immediate8 => 1,
                
                ArgumentTypes::Immediate16 => 2
            };
        }

        size
    }
}

pub struct Instructions {
    ilist: HashMap<&'static str, Instruction>
}

impl Instructions {
    pub fn new() -> Self {
        let mut me = Self { ilist: HashMap::new() };

        me.ilist.insert("nop", Instruction { name: "nop", opcode: 0, args: vec![] });
        me.ilist.insert("halt", Instruction { name: "halt", opcode: 1, args: vec![] });
        me.ilist.insert("radd", Instruction { name: "add", opcode: 2, args: vec![ArgumentTypes::Register32, ArgumentTypes::Register32] });
        me.ilist.insert("iadd", Instruction { name: "add", opcode: 3, args: vec![ArgumentTypes::Immediate32, ArgumentTypes::Register32] });
        me.ilist.insert("loadmd", Instruction { name: "loadm dw", opcode: 4, args: vec![ArgumentTypes::AbsPointer, ArgumentTypes::Register32] });
        me.ilist.insert("loadid", Instruction { name: "loadi dw", opcode: 5, args: vec![ArgumentTypes::Immediate32, ArgumentTypes::Register32] });

        me
    }
    pub fn get_opcode(&self, name: &str) -> Option<u16> {
        Some(self.ilist.get(name)?.opcode)
    }
    pub fn get_instruction(&self, opcode: u16) -> Option<&Instruction> {
        self.ilist.values().find(|i| i.opcode == opcode)
    }
}
