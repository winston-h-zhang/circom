use super::ir_interface::*;
use crate::translating_traits::*;
use code_producers::rust_elements::*;
use code_producers::wasm_elements::*;

#[derive(Clone)]
pub enum LogBucketArg {
    LogExp(InstructionPointer),
    LogStr(usize),
}
impl LogBucketArg {
    pub fn get_mut_arg_logexp(&mut self) -> &mut InstructionPointer {
        match self {
            LogBucketArg::LogExp(arg) => arg,
            LogBucketArg::LogStr(_) => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct LogBucket {
    pub line: usize,
    pub message_id: usize,
    pub argsprint: Vec<LogBucketArg>,
}

impl IntoInstruction for LogBucket {
    fn into_instruction(self) -> Instruction {
        Instruction::Log(self)
    }
}

impl Allocate for LogBucket {
    fn allocate(self) -> InstructionPointer {
        InstructionPointer::new(self.into_instruction())
    }
}

impl ObtainMeta for LogBucket {
    fn get_line(&self) -> usize {
        self.line
    }
    fn get_message_id(&self) -> usize {
        self.message_id
    }
}

impl ToString for LogBucket {
    fn to_string(&self) -> String {
        let line = self.line.to_string();
        let template_id = self.message_id.to_string();
        let mut ret = String::new();
        for print in self.argsprint.clone() {
            if let LogBucketArg::LogExp(exp) = print {
                let print = exp.to_string();
                let log = format!(
                    "LOG(line: {},template_id: {},evaluate: {})",
                    line, template_id, print
                );
                ret = ret + &log;
            }
        }
        ret
    }
}

impl WriteWasm for LogBucket {
    fn produce_wasm(&self, producer: &WASMProducer) -> Vec<String> {
        use code_producers::wasm_elements::wasm_code_generator::*;
        let mut instructions = vec![];
        if producer.needs_comments() {
            instructions.push(";; log bucket".to_string());
        }
        for logarg in self.argsprint.clone() {
            match &logarg {
                LogBucketArg::LogExp(exp) => {
                    let mut instructions_print = exp.produce_wasm(producer);
                    instructions.append(&mut instructions_print);
                    instructions.push(call("$copyFr2SharedRWMemory"));
                    instructions.push(call("$showSharedRWMemory"));
                }
                LogBucketArg::LogStr(stringid) => {
                    let pos = producer.get_string_list_start()
                        + stringid * producer.get_size_of_message_in_bytes();
                    instructions.push(set_constant(&pos.to_string()));
                    instructions.push(call("$buildLogMessage"));
                    instructions.push(call("$writeBufferMessage"));
                }
            }
        }
        // add nl
        instructions.push(set_constant(
            &producer.get_message_buffer_start().to_string(),
        ));
        instructions.push(set_constant("0x0000000a"));
        instructions.push(store32(None)); // stores \n000
        instructions.push(set_constant(
            &producer.get_message_buffer_counter_position().to_string(),
        ));
        instructions.push(set_constant("0"));
        instructions.push(store32(None));
        instructions.push(call("$writeBufferMessage"));
        if producer.needs_comments() {
            instructions.push(";; end of log bucket".to_string());
        }
        instructions
    }
}

impl WriteRust for LogBucket {
    fn produce_rust(&self, producer: &RustProducer, parallel: Option<bool>) -> (Vec<String>, String) {
        todo!("log")
    }
}
