use crate::{objgen::ObjectFormat, symbols::Instructions};

pub struct Objdump {
    object: ObjectFormat
}

impl Objdump {
    pub fn new(object: ObjectFormat) -> Self {
        Self { object }
    }
    pub fn get_disassembly(&self) -> Result<String, String> {
        let instructions = Instructions::new();

        let mut result = String::new();

        for (sec_name, sec) in self.object.sections.iter() {
            if sec.binary_section || sec.instructions.len() == 0 {
                continue;
            }

            result += &format!("Section '{}':\n", sec_name);

            for (i, instruction) in sec.instructions.iter().enumerate() {
                match sec.labels.iter().find(|(_, l)| l.ptr_instr == (i as u64)) {
                    Some((l_name, _)) => {
                        result += &format!("\n  <'{}'>:\n", l_name);
                    }
                    None => {}
                };
                let sym = match instructions.get_instruction(instruction.opcode) {
                    Some(s) => s,
                    None => {
                        return Err(format!("No instruction with opcode '{}' exists!", instruction.opcode))
                    }
                };
                result += &format!("\t{:#04x}: {} ", instruction.opcode, sym.name);

                result += &instruction.get_args();

                result += "\n";

                // final format:
                //      opc: nam a0 a1 \n
            }
        }

        Ok(result)
    }
}