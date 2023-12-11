use std::collections::HashMap;

pub struct Conditions {
    conditions: HashMap<&'static str, u8>
}

impl Conditions {
    pub fn new() -> Self {
        let mut me = Self { conditions: HashMap::new() };

        // Math flags
        me.conditions.insert("OV", 0);
        me.conditions.insert("CR", 1);
        me.conditions.insert("NG", 2);
        me.conditions.insert("ZR", 3);

        me.conditions.insert("NV", 0 + 32);
        me.conditions.insert("NC", 1 + 32);
        me.conditions.insert("NN", 2 + 32);
        me.conditions.insert("NZ", 3 + 32);

        // Status register flags
        me.conditions.insert("ILF", 64);
        me.conditions.insert("HLF", 65);
        me.conditions.insert("IDF", 66);

        me.conditions.insert("NILF", 64 + 32);
        me.conditions.insert("NHLF", 65 + 32);
        me.conditions.insert("NIDF", 66 + 32);

        me
    }

    pub fn get_condition(&self, name: &str) -> Option<&u8> {
        self.conditions.get(name)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArgumentTypes {
    AbsPointer, RelPointer,
    Register32, Register16, Register8,
    Immediate32, Immediate16, Immediate8,
    FloatingPoint, Condition
}

impl ArgumentTypes {
    pub fn get_size(&self) -> usize {
        match self {
            ArgumentTypes::AbsPointer |
            ArgumentTypes::RelPointer |
            ArgumentTypes::FloatingPoint |
            ArgumentTypes::Immediate32 => 4,

            ArgumentTypes::Register16 |
            ArgumentTypes::Register32 |
            ArgumentTypes::Register8 |
            ArgumentTypes::Immediate8 |
            ArgumentTypes::Condition => 1,
            
            ArgumentTypes::Immediate16 => 2
        }
    }
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
            size += arg.get_size();
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

        me.ilist.insert("madd", Instruction { name: "add", opcode: 6, args: vec![ArgumentTypes::AbsPointer, ArgumentTypes::Register32] });
        me.ilist.insert("loadmb", Instruction { name: "loadm b", opcode: 7, args: vec![ArgumentTypes::AbsPointer, ArgumentTypes::Register8] });
        me.ilist.insert("loadib", Instruction { name: "loadi b", opcode: 8, args: vec![ArgumentTypes::Immediate8, ArgumentTypes::Register8] });
        me.ilist.insert("jmp", Instruction { name: "jmp", opcode: 9, args: vec![ArgumentTypes::AbsPointer] });
        me.ilist.insert("jpc", Instruction { name: "jpc", opcode: 10, args: vec![ArgumentTypes::AbsPointer, ArgumentTypes::Condition] });
        me.ilist.insert("call", Instruction { name: "call", opcode: 11, args: vec![ArgumentTypes::AbsPointer] });

        me.ilist.insert("jpr", Instruction { name: "jpr", opcode: 12, args: vec![ArgumentTypes::RelPointer] });
        me.ilist.insert("jrc", Instruction { name: "jrc", opcode: 13, args: vec![ArgumentTypes::RelPointer, ArgumentTypes::Condition] });
        me.ilist.insert("callr", Instruction { name: "callr", opcode: 14, args: vec![ArgumentTypes::RelPointer] });
        me.ilist.insert("push", Instruction { name: "push", opcode: 15, args: vec![ArgumentTypes::Register32] });
        me.ilist.insert("pop", Instruction { name: "pop", opcode: 16, args: vec![ArgumentTypes::Register32] });
        me.ilist.insert("ret", Instruction { name: "ret", opcode: 17, args: vec![] });

        me.ilist.insert("movrd", Instruction { name: "movrd", opcode: 18, args: vec![ArgumentTypes::Register32, ArgumentTypes::Register32] });
        me.ilist.insert("movrw", Instruction { name: "movrw", opcode: 19, args: vec![ArgumentTypes::Register16, ArgumentTypes::Register16] });
        me.ilist.insert("movrb", Instruction { name: "movrb", opcode: 20, args: vec![ArgumentTypes::Register8, ArgumentTypes::Register8] });
        me.ilist.insert("int", Instruction { name: "int", opcode: 21, args: vec![ArgumentTypes::Immediate8] });
        me.ilist.insert("isub", Instruction { name: "isub", opcode: 22, args: vec![ArgumentTypes::Immediate32, ArgumentTypes::Register32] });
        me.ilist.insert("msub", Instruction { name: "msub", opcode: 23, args: vec![ArgumentTypes::AbsPointer, ArgumentTypes::Register32] });

        me.ilist.insert("rsub", Instruction { name: "rsub", opcode: 24, args: vec![ArgumentTypes::Register32, ArgumentTypes::Register32] });
        me.ilist.insert("ngi", Instruction { name: "ngi", opcode: 25, args: vec![ArgumentTypes::Register32] });
        me.ilist.insert("rmulsd", Instruction { name: "rmulsd", opcode: 26, args: vec![ArgumentTypes::Register32, ArgumentTypes::Register32] });
        me.ilist.insert("rdivsd", Instruction { name: "rdivsd", opcode: 27, args: vec![ArgumentTypes::Register32, ArgumentTypes::Register32] });
        me.ilist.insert("rmulud", Instruction { name: "rmulud", opcode: 28, args: vec![ArgumentTypes::Register32, ArgumentTypes::Register32] });
        me.ilist.insert("rdivud", Instruction { name: "rdivud", opcode: 29, args: vec![ArgumentTypes::Register32, ArgumentTypes::Register32] });

        me.ilist.insert("imulsd", Instruction { name: "imulsd", opcode: 30, args: vec![ArgumentTypes::Immediate32, ArgumentTypes::Register32] });
        me.ilist.insert("idivsd", Instruction { name: "idivsd", opcode: 31, args: vec![ArgumentTypes::Immediate32, ArgumentTypes::Register32] });
        me.ilist.insert("imulud", Instruction { name: "imulud", opcode: 32, args: vec![ArgumentTypes::Immediate32, ArgumentTypes::Register32] });
        me.ilist.insert("idivud", Instruction { name: "idivud", opcode: 33, args: vec![ArgumentTypes::Immediate32, ArgumentTypes::Register32] });
        me.ilist.insert("cvsdf", Instruction { name: "cvsdf", opcode: 34, args: vec![ArgumentTypes::Register32] });
        me.ilist.insert("cvfsd", Instruction { name: "cvfsd", opcode: 35, args: vec![ArgumentTypes::Register32] });

        me
    }
    pub fn get_opcode(&self, name: &str) -> Option<u16> {
        Some(self.ilist.get(name)?.opcode)
    }
    pub fn get_instruction(&self, opcode: u16) -> Option<&Instruction> {
        self.ilist.values().find(|i| i.opcode == opcode)
    }
}
