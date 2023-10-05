use super::ir_interface::*;
use crate::translating_traits::*;
use code_producers::rust_elements::*;
use code_producers::wasm_elements::*;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum OperatorType {
    Mul,
    Div,
    Add,
    Sub,
    Pow,
    IntDiv,
    Mod,
    ShiftL,
    ShiftR,
    LesserEq,
    GreaterEq,
    Lesser,
    Greater,
    Eq(usize),
    NotEq,
    BoolOr,
    BoolAnd,
    BitOr,
    BitAnd,
    BitXor,
    PrefixSub,
    BoolNot,
    Complement,
    ToAddress,
    MulAddress,
    AddAddress,
}

impl OperatorType {
    pub fn is_address_op(&self) -> bool {
        *self == OperatorType::ToAddress
            || *self == OperatorType::AddAddress
            || *self == OperatorType::MulAddress
    }

    pub fn is_multiple_eq(&self) -> bool {
        match self {
            OperatorType::Eq(n) => *n > 1,
            _ => false,
        }
    }
}

impl ToString for OperatorType {
    fn to_string(&self) -> String {
        use OperatorType::*;
        if let Eq(n) = self {
            format!("EQ({})", n)
        } else {
            match self {
                Mul => "MUL",
                Div => "DIV",
                Add => "ADD",
                Sub => "SUB",
                Pow => "POW",
                IntDiv => "INT_DIV",
                Mod => "MOD",
                ShiftL => "SHIFT_L",
                ShiftR => "SHIFT_R",
                LesserEq => "LESSER_EQ",
                GreaterEq => "GREATER_EQ",
                Lesser => "LESSER",
                Greater => "GREATER",
                NotEq => "NOT_EQ",
                BoolOr => "BOOL_OR",
                BoolAnd => "BOOL_AND",
                BitOr => "BITOR",
                BitAnd => "BITAND",
                BitXor => "BITXOR",
                PrefixSub => "PREFIX_SUB",
                BoolNot => "BOOL_NOT",
                Complement => "COMPLEMENT",
                ToAddress => "TO_ADDRESS",
                MulAddress => "MUL_ADDRESS",
                AddAddress => "ADD_ADDRESS",
                _ => "",
            }
            .to_string()
        }
    }
}

#[derive(Clone)]
pub struct ComputeBucket {
    pub line: usize,
    pub message_id: usize,
    pub op: OperatorType,
    pub op_aux_no: usize,
    pub stack: Vec<InstructionPointer>,
}

impl IntoInstruction for ComputeBucket {
    fn into_instruction(self) -> Instruction {
        Instruction::Compute(self)
    }
}

impl Allocate for ComputeBucket {
    fn allocate(self) -> InstructionPointer {
        InstructionPointer::new(self.into_instruction())
    }
}

impl ObtainMeta for ComputeBucket {
    fn get_line(&self) -> usize {
        self.line
    }
    fn get_message_id(&self) -> usize {
        self.message_id
    }
}

impl ToString for ComputeBucket {
    fn to_string(&self) -> String {
        let line = self.line.to_string();
        let template_id = self.message_id.to_string();
        let op = self.op.to_string();
        let op_number = self.op_aux_no.to_string();
        let mut stack = "".to_string();
        for i in &self.stack {
            stack = format!("{}{};", stack, i.to_string());
        }
        format!(
            "COMPUTE(line:{},template_id:{},op_number:{},op:{},stack:{})",
            line, template_id, op_number, op, stack
        )
    }
}
impl WriteWasm for ComputeBucket {
    fn produce_wasm(&self, producer: &WASMProducer) -> Vec<String> {
        use code_producers::wasm_elements::wasm_code_generator::*;
        let mut instructions = vec![];
        if producer.needs_comments() {
            instructions.push(";; compute bucket".to_string());
        }
        match &self.op {
            OperatorType::AddAddress => {}
            OperatorType::MulAddress => {}
            OperatorType::ToAddress => {}
            _ => {
                //address of the result for the Fr operations
                instructions.push(get_local(producer.get_expaux_tag()));
                let size = self.op_aux_no * producer.get_size_32_bits_in_memory() * 4;
                instructions.push(set_constant(&size.to_string()));
                instructions.push(add32());
            }
        }
        for e in &self.stack {
            let mut instructions_exp = e.produce_wasm(producer);
            instructions.append(&mut instructions_exp);
        }
        if producer.needs_comments() {
            instructions.push(format!(";; OP({})", self.op.to_string()));
        }
        match &self.op {
            OperatorType::AddAddress => {
                instructions.push(add32());
            }
            OperatorType::MulAddress => {
                instructions.push(mul32());
            }
            OperatorType::ToAddress => {
                instructions.push(call("$Fr_toInt"));
            }
            _ => {
                match self.op {
                    OperatorType::Add => {
                        instructions.push(call("$Fr_add")); // Result, Argument, Argument
                    }
                    OperatorType::Div => {
                        instructions.push(call("$Fr_div")); // Result, Argument, Argument
                    }
                    OperatorType::Mul => {
                        instructions.push(call("$Fr_mul")); // Result, Argument, Argument
                    }
                    OperatorType::Sub => {
                        instructions.push(call("$Fr_sub")); // Result, Argument, Argument
                    }
                    OperatorType::Pow => {
                        instructions.push(call("$Fr_pow"));
                    }
                    OperatorType::IntDiv => {
                        instructions.push(call("$Fr_idiv"));
                    }
                    OperatorType::Mod => {
                        instructions.push(call("$Fr_mod"));
                    }
                    OperatorType::ShiftL => {
                        instructions.push(call("$Fr_shl"));
                    }
                    OperatorType::ShiftR => {
                        instructions.push(call("$Fr_shr"));
                    }
                    OperatorType::LesserEq => {
                        instructions.push(call("$Fr_leq"));
                    }
                    OperatorType::GreaterEq => {
                        instructions.push(call("$Fr_geq"));
                    }
                    OperatorType::Lesser => {
                        instructions.push(call("$Fr_lt"));
                    }
                    OperatorType::Greater => {
                        instructions.push(call("$Fr_gt"));
                    }
                    OperatorType::Eq(n) => {
                        assert!(n != 0);
                        if n == 1 {
                            instructions.push(call("$Fr_eq"));
                        } else {
                            instructions.push(set_local(producer.get_aux_2_tag()));
                            instructions.push(set_local(producer.get_aux_1_tag()));
                            instructions.push(set_local(producer.get_aux_0_tag()));
                            instructions.push(set_constant(&n.to_string()));
                            instructions.push(set_local(producer.get_counter_tag()));
                            instructions.push(add_block());
                            instructions.push(add_loop());
                            instructions.push(get_local(producer.get_aux_0_tag()));
                            instructions.push(get_local(producer.get_aux_1_tag()));
                            instructions.push(get_local(producer.get_aux_2_tag()));
                            instructions.push(call("$Fr_eq"));
                            instructions.push(get_local(producer.get_aux_0_tag()));
                            instructions.push(call("$Fr_isTrue"));
                            instructions.push(eqz32());
                            instructions.push(br_if("1"));
                            instructions.push(get_local(producer.get_counter_tag()));
                            instructions.push(set_constant("1"));
                            instructions.push(sub32());
                            instructions.push(tee_local(producer.get_counter_tag()));
                            instructions.push(eqz32());
                            instructions.push(br_if("1"));
                            instructions.push(get_local(producer.get_aux_1_tag()));
                            let s = producer.get_size_32_bits_in_memory() * 4;
                            instructions.push(set_constant(&s.to_string()));
                            instructions.push(add32());
                            instructions.push(set_local(producer.get_aux_1_tag()));
                            instructions.push(get_local(producer.get_aux_2_tag()));
                            instructions.push(set_constant(&s.to_string()));
                            instructions.push(add32());
                            instructions.push(set_local(producer.get_aux_2_tag()));
                            instructions.push(br("0"));
                            instructions.push(add_end());
                            instructions.push(add_end());
                        }
                    }
                    OperatorType::NotEq => {
                        instructions.push(call("$Fr_neq"));
                    }
                    OperatorType::BoolOr => {
                        instructions.push(call("$Fr_lor"));
                    }
                    OperatorType::BoolAnd => {
                        instructions.push(call("$Fr_land"));
                    }
                    OperatorType::BitOr => {
                        instructions.push(call("$Fr_bor"));
                    }
                    OperatorType::BitAnd => {
                        instructions.push(call("$Fr_band"));
                    }
                    OperatorType::BitXor => {
                        instructions.push(call("$Fr_bxor"));
                    }
                    OperatorType::PrefixSub => {
                        instructions.push(call("$Fr_neg"));
                    }
                    OperatorType::BoolNot => {
                        instructions.push(call("$Fr_lnot"));
                    }
                    OperatorType::Complement => {
                        instructions.push(call("$Fr_bnot"));
                    }
                    _ => (), //$Fr_inv? Does not exists
                }
                instructions.push(get_local(producer.get_expaux_tag()));
                let size = self.op_aux_no * producer.get_size_32_bits_in_memory() * 4;
                instructions.push(set_constant(&size.to_string()));
                instructions.push(add32());
            }
        }
        if producer.needs_comments() {
            instructions.push(";; end of compute bucket".to_string());
        }
        instructions
    }
}

impl WriteRust for ComputeBucket {
    fn produce_rust(&self, producer: &RustProducer, parallel: Option<bool>) -> (Vec<String>, String) {
        fn build_op(op_type: OperatorType, rhs: Vec<String>) -> String {
            match op_type {
                OperatorType::Add => format!("({} + {})", rhs[0], rhs[1]),
                OperatorType::Sub => format!("({} - {})", rhs[0], rhs[1]),
                OperatorType::Mul => format!("({} * {})", rhs[0], rhs[1]),
                OperatorType::Div => format!("({} / {})", rhs[0], rhs[1]),
                OperatorType::Pow => todo!(),
                OperatorType::IntDiv => todo!(),
                OperatorType::Mod => todo!(),
                OperatorType::ShiftL => format!("field::shl({}, {})", rhs[0], rhs[1]),
                OperatorType::ShiftR => format!("field::shr({}, {})", rhs[0], rhs[1]),
                OperatorType::LesserEq => todo!(),
                OperatorType::GreaterEq => todo!(),
                OperatorType::Lesser => format!("field::from_bool(field::lt({}, {}))", rhs[0], rhs[1]),
                OperatorType::Greater => todo!(),
                OperatorType::Eq(_) => format!("field::from_bool({} == {})", rhs[0], rhs[1]),
                OperatorType::NotEq => todo!(),
                OperatorType::BoolOr => todo!(),
                OperatorType::BoolAnd => todo!(),
                OperatorType::BitOr => todo!(),
                OperatorType::BitAnd => format!("field::bit_and({}, {})", rhs[0], rhs[1]),
                OperatorType::BitXor => todo!(),
                OperatorType::PrefixSub => todo!(),
                OperatorType::BoolNot => todo!(),
                OperatorType::Complement => todo!(),
                _ => unreachable!(),
            }
        }

        let mut compute_rust = vec![];
        let mut operands = vec![];

        for instr in &self.stack {
            let (mut instr_rust, operand) = instr.produce_rust(producer, parallel);
            operands.push(operand);
            compute_rust.append(&mut instr_rust);
        }

        let result;
        match &self.op {
            OperatorType::AddAddress => {
                result = format!("({} + {})", operands[0], operands[1]);
            }
            OperatorType::MulAddress => {
                result = format!("({} * {})", operands[0], operands[1]);
            }
            OperatorType::ToAddress => {
                result = format!("(field::to_u64({}).unwrap() as usize)", operands.join(", "));
            }

            _ => {
                result = format!("expaux[{}]", self.op_aux_no);
                compute_rust.push(format!(
                    "{} = {}; // circom line {}",
                    result,
                    build_op(self.op, operands),
                    self.line
                ));
            }
        }
        // compute_rust.push(format!("// end of compute with result {}", result));
        (compute_rust, result)
    }
}
