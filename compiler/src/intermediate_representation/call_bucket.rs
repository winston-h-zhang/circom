use super::ir_interface::*;
use crate::translating_traits::*;
use code_producers::rust_elements::*;
use code_producers::wasm_elements::*;

#[derive(Clone)]
pub struct FinalData {
    // greater than one only with signals.
    pub context: InstrContext,
    pub dest_is_output: bool,
    pub dest_address_type: AddressType,
    pub dest: LocationRule,
}

#[derive(Clone)]
pub enum ReturnType {
    Intermediate { op_aux_no: usize },
    Final(FinalData),
}

#[derive(Clone)]
pub struct CallBucket {
    pub line: usize,
    pub message_id: usize,
    pub symbol: String,
    pub argument_types: Vec<InstrContext>,
    pub arguments: InstructionList,
    pub arena_size: usize,
    pub return_info: ReturnType,
}

impl IntoInstruction for CallBucket {
    fn into_instruction(self) -> Instruction {
        Instruction::Call(self)
    }
}

impl Allocate for CallBucket {
    fn allocate(self) -> InstructionPointer {
        InstructionPointer::new(self.into_instruction())
    }
}

impl ObtainMeta for CallBucket {
    fn get_line(&self) -> usize {
        self.line
    }
    fn get_message_id(&self) -> usize {
        self.message_id
    }
}

impl ToString for CallBucket {
    fn to_string(&self) -> String {
        let line = self.line.to_string();
        let template_id = self.message_id.to_string();
        let ret = match &self.return_info {
            ReturnType::Intermediate { op_aux_no } => {
                format!("Intermediate({})", op_aux_no)
            }
            _ => {
                "Final".to_string()
            }
        };
        let mut args = "".to_string();
        for i in &self.arguments {
            args = format!("{}{},", args, i.to_string());
        }
        format!(
            "CALL(line:{},template_id:{},id:{},return_type:{},args:{})",
            line, template_id, self.symbol, ret, args
        )
    }
}

impl WriteWasm for CallBucket {
    fn produce_wasm(&self, producer: &WASMProducer) -> Vec<String> {
        use code_producers::wasm_elements::wasm_code_generator::*;
        let mut instructions = vec![];
        if producer.needs_comments() {
            instructions.push(";; call bucket".to_string());
        }
        if !self.arguments.is_empty() {
            let local_info_size_u32 = producer.get_local_info_size_u32();
            instructions.push(set_constant("0"));
            instructions.push(load32(None)); // current stack size
            let var_start = local_info_size_u32 * 4; // starts after local info
            if local_info_size_u32 != 0 {
                instructions.push(set_constant(&var_start.to_string()));
                instructions.push(add32());
            }
            instructions.push(set_local(producer.get_call_lvar_tag()));
            let mut count = 0;
            let mut i = 0;
            for p in &self.arguments {
                if producer.needs_comments() {
                    instructions.push(format!(";; copying argument {}", i));
                }
                instructions.push(get_local(producer.get_call_lvar_tag()));
                instructions.push(set_constant(&count.to_string()));
                instructions.push(add32());
                if self.argument_types[i].size > 1 {
                    instructions.push(set_local(producer.get_store_aux_1_tag()));
                }
                let mut instructions_arg = p.produce_wasm(producer);
                instructions.append(&mut instructions_arg);
                if self.argument_types[i].size == 1 {
                    instructions.push(call("$Fr_copy"));
                } else {
                    instructions.push(set_local(producer.get_store_aux_2_tag()));
                    instructions.push(set_constant(&self.argument_types[i].size.to_string()));
                    instructions.push(set_local(producer.get_copy_counter_tag()));
                    instructions.push(add_block());
                    instructions.push(add_loop());
                    instructions.push(get_local(producer.get_copy_counter_tag()));
                    instructions.push(eqz32());
                    instructions.push(br_if("1"));
                    instructions.push(get_local(producer.get_store_aux_1_tag()));
                    instructions.push(get_local(producer.get_store_aux_2_tag()));
                    instructions.push(call("$Fr_copy"));
                    instructions.push(get_local(producer.get_copy_counter_tag()));
                    instructions.push(set_constant("1"));
                    instructions.push(sub32());
                    instructions.push(set_local(producer.get_copy_counter_tag()));
                    instructions.push(get_local(producer.get_store_aux_1_tag()));
                    let s = producer.get_size_32_bits_in_memory() * 4;
                    instructions.push(set_constant(&s.to_string()));
                    instructions.push(add32());
                    instructions.push(set_local(producer.get_store_aux_1_tag()));
                    instructions.push(get_local(producer.get_store_aux_2_tag()));
                    instructions.push(set_constant(&s.to_string()));
                    instructions.push(add32());
                    instructions.push(set_local(producer.get_store_aux_2_tag()));
                    instructions.push(br("0"));
                    instructions.push(add_end());
                    instructions.push(add_end());
                }
                if producer.needs_comments() {
                    instructions.push(format!(";; end copying argument {}", i));
                }
                count += self.argument_types[i].size * 4 * producer.get_size_32_bits_in_memory();
                i += 1;
            }
        }
        match &self.return_info {
            ReturnType::Intermediate { op_aux_no } => {
                instructions.push(get_local(producer.get_expaux_tag()));
                instructions.push(set_constant(&op_aux_no.to_string()));
                instructions.push(add32());
                instructions.push(set_constant("1"));
                instructions.push(call(&format!("${}", self.symbol)));
                instructions.push(tee_local(producer.get_merror_tag()));
                instructions.push(add_if());
                instructions.push(call("$printErrorMessage"));
                instructions.push(get_local(producer.get_merror_tag()));
                instructions.push(add_return());
                instructions.push(add_end());
                instructions.push(get_local(producer.get_expaux_tag()));
                instructions.push(set_constant(&op_aux_no.to_string()));
                instructions.push(add32());
            }
            ReturnType::Final(data) => {
                let mut my_template_header = Option::<String>::None;
                match &data.dest {
                    LocationRule::Indexed {
                        location,
                        template_header,
                    } => {
                        if producer.needs_comments() {
                            instructions.push(";; getting result address".to_string());
                        }
                        let mut instructions_dest = location.produce_wasm(producer);
                        instructions.append(&mut instructions_dest);
                        let size = producer.get_size_32_bits_in_memory() * 4;
                        instructions.push(set_constant(&size.to_string()));
                        instructions.push(mul32());
                        match &data.dest_address_type {
                            AddressType::Variable => {
                                instructions.push(get_local(producer.get_lvar_tag()));
                            }
                            AddressType::Signal => {
                                instructions.push(get_local(producer.get_signal_start_tag()));
                            }
                            AddressType::SubcmpSignal { cmp_address, .. } => {
                                my_template_header = template_header.clone();
                                instructions.push(get_local(producer.get_offset_tag()));
                                instructions.push(set_constant(
                                    &producer.get_sub_component_start_in_component().to_string(),
                                ));
                                instructions.push(add32());
                                let mut instructions_sci = cmp_address.produce_wasm(producer);
                                instructions.append(&mut instructions_sci);
                                instructions.push(set_constant("4")); //size in byte of i32
                                instructions.push(mul32());
                                instructions.push(add32());
                                instructions.push(load32(None)); //subcomponent block
                                instructions.push(set_local(producer.get_sub_cmp_tag()));
                                instructions.push(get_local(producer.get_sub_cmp_tag()));
                                instructions.push(set_constant(
                                    &producer.get_signal_start_address_in_component().to_string(),
                                ));
                                instructions.push(add32());
                                instructions.push(load32(None)); //subcomponent start_of_signals
                            }
                        }
                        instructions.push(add32());
                    }
                    LocationRule::Mapped {
                        signal_code,
                        indexes,
                    } => {
                        match &data.dest_address_type {
                            AddressType::SubcmpSignal { cmp_address, .. } => {
                                if producer.needs_comments() {
                                    instructions.push(";; is subcomponent".to_string());
                                }
                                instructions.push(get_local(producer.get_offset_tag()));
                                instructions.push(set_constant(
                                    &producer.get_sub_component_start_in_component().to_string(),
                                ));
                                instructions.push(add32());
                                let mut instructions_sci = cmp_address.produce_wasm(producer);
                                instructions.append(&mut instructions_sci);
                                instructions.push(set_constant("4")); //size in byte of i32
                                instructions.push(mul32());
                                instructions.push(add32());
                                instructions.push(load32(None)); //subcomponent block
                                instructions.push(set_local(producer.get_sub_cmp_tag()));
                                instructions.push(get_local(producer.get_sub_cmp_tag()));
                                instructions.push(load32(None)); // get template id                     A
                                instructions.push(set_constant("4")); //size in byte of i32
                                instructions.push(mul32());
                                instructions.push(load32(Some(
                                    &producer
                                        .get_template_instance_to_io_signal_start()
                                        .to_string(),
                                ))); // get position in component io signal to info list
                                let signal_code_in_bytes = signal_code * 4; //position in the list of the signal code
                                instructions.push(load32(Some(&signal_code_in_bytes.to_string()))); // get where the info of this signal is
                                                                                                    //now we have first the offset and then the all size dimensions but the last one
                                if indexes.len() <= 1 {
                                    instructions.push(load32(None)); // get signal offset (it is already the actual one in memory);
                                    if indexes.len() == 1 {
                                        let mut instructions_idx0 =
                                            indexes[0].produce_wasm(producer);
                                        instructions.append(&mut instructions_idx0);
                                        let size = producer.get_size_32_bits_in_memory() * 4;
                                        instructions.push(set_constant(&size.to_string()));
                                        instructions.push(mul32());
                                        instructions.push(add32());
                                    }
                                } else {
                                    instructions.push(set_local(producer.get_io_info_tag()));
                                    instructions.push(get_local(producer.get_io_info_tag()));
                                    instructions.push(load32(None)); // get signal offset (it is already the actual one in memory);
                                                                     // compute de move with 2 or more dimensions
                                    let mut instructions_idx0 = indexes[0].produce_wasm(producer);
                                    instructions.append(&mut instructions_idx0); // start with dimension 0
                                    for i in 1..indexes.len() {
                                        instructions.push(get_local(producer.get_io_info_tag()));
                                        let offsetdim = 4 * i;
                                        instructions.push(load32(Some(&offsetdim.to_string()))); // get size of ith dimension
                                        instructions.push(mul32()); // multiply the current move by size of the ith dimension
                                        let mut instructions_idxi =
                                            indexes[i].produce_wasm(producer);
                                        instructions.append(&mut instructions_idxi);
                                        instructions.push(add32()); // add move upto dimension i
                                    }
                                    //we have the total move; and is multiplied by the size of memory Fr in bytes
                                    let size = producer.get_size_32_bits_in_memory() * 4;
                                    instructions.push(set_constant(&size.to_string()));
                                    instructions.push(mul32()); // We have the total move in bytes
                                    instructions.push(add32()); // add to the offset of the signal
                                }
                                instructions.push(get_local(producer.get_sub_cmp_tag()));
                                instructions.push(set_constant(
                                    &producer.get_signal_start_address_in_component().to_string(),
                                ));
                                instructions.push(add32());
                                instructions.push(load32(None)); //subcomponent start_of_signals
                                instructions.push(add32()); // we get the position of the signal (with indexes) in memory
                            }
                            _ => {
                                assert!(false);
                            }
                        }
                    }
                }
                instructions.push(set_constant(&data.context.size.to_string()));
                instructions.push(call(&format!("${}", self.symbol)));
                instructions.push(tee_local(producer.get_merror_tag()));
                instructions.push(add_if());
                instructions.push(set_constant(&self.message_id.to_string()));
                instructions.push(set_constant(&self.line.to_string()));
                instructions.push(call("$buildBufferMessage"));
                instructions.push(call("$printErrorMessage"));
                instructions.push(get_local(producer.get_merror_tag()));
                instructions.push(add_return());
                instructions.push(add_end());
                match &data.dest_address_type {
                    AddressType::SubcmpSignal { .. } => {
                        // if subcomponent input check if run needed
                        if producer.needs_comments() {
                            instructions.push(";; decrease counter".to_string());
                            // by self.context.size
                        }
                        instructions.push(get_local(producer.get_sub_cmp_tag()));
                        instructions.push(get_local(producer.get_sub_cmp_tag()));
                        instructions.push(load32(Some(
                            &producer
                                .get_input_counter_address_in_component()
                                .to_string(),
                        ))); //remaining inputs to be set
                        instructions.push(set_constant(&data.context.size.to_string()));
                        instructions.push(sub32());
                        instructions.push(store32(Some(
                            &producer
                                .get_input_counter_address_in_component()
                                .to_string(),
                        ))); // update remaining inputs to be set
                        if producer.needs_comments() {
                            instructions.push(";; check if run is needed".to_string());
                        }
                        instructions.push(get_local(producer.get_sub_cmp_tag()));
                        instructions.push(load32(Some(
                            &producer
                                .get_input_counter_address_in_component()
                                .to_string(),
                        )));
                        instructions.push(eqz32());
                        instructions.push(add_if());
                        if producer.needs_comments() {
                            instructions.push(";; run sub component".to_string());
                        }
                        instructions.push(get_local(producer.get_sub_cmp_tag()));
                        match &data.dest {
                            LocationRule::Indexed { .. } => {
                                if let Some(name) = &my_template_header {
                                    instructions.push(call(&format!("${}_run", name)));
                                    instructions.push(tee_local(producer.get_merror_tag()));
                                    instructions.push(add_if());
                                    instructions.push(set_constant(&self.message_id.to_string()));
                                    instructions.push(set_constant(&self.line.to_string()));
                                    instructions.push(call("$buildBufferMessage"));
                                    instructions.push(call("$printErrorMessage"));
                                    instructions.push(get_local(producer.get_merror_tag()));
                                    instructions.push(add_return());
                                    instructions.push(add_end());
                                } else {
                                    assert!(false);
                                }
                            }
                            LocationRule::Mapped { .. } => {
                                instructions.push(get_local(producer.get_sub_cmp_tag()));
                                instructions.push(load32(None)); // get template id
                                instructions.push(call_indirect(
                                    "$runsmap",
                                    "(type $_t_i32ri32)",
                                ));
                                instructions.push(tee_local(producer.get_merror_tag()));
                                instructions.push(add_if());
                                instructions.push(set_constant(&self.message_id.to_string()));
                                instructions.push(set_constant(&self.line.to_string()));
                                instructions.push(call("$buildBufferMessage"));
                                instructions.push(call("$printErrorMessage"));
                                instructions.push(get_local(producer.get_merror_tag()));
                                instructions.push(add_return());
                                instructions.push(add_end());
                            }
                        }
                        if producer.needs_comments() {
                            instructions.push(";; end run sub component".to_string());
                        }
                        instructions.push(add_end());
                    }
                    _ => (),
                }
            }
        }
        if producer.needs_comments() {
            instructions.push(";; end call bucket".to_string());
        }
        instructions
    }
}

impl WriteRust for CallBucket {
    fn produce_rust(&self, producer: &RustProducer, parallel: Option<bool>) -> (Vec<String>, String) {
        todo!("call")
    }
}
