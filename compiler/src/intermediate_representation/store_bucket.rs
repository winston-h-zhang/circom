use super::ir_interface::*;
use crate::translating_traits::*;
use code_producers::rust_elements::*;
use code_producers::wasm_elements::*;

#[derive(Clone)]
pub struct StoreBucket {
    pub line: usize,
    pub message_id: usize,
    pub context: InstrContext,
    pub dest_is_output: bool,
    pub dest_address_type: AddressType,
    pub dest: LocationRule,
    pub src: InstructionPointer,
}

impl IntoInstruction for StoreBucket {
    fn into_instruction(self) -> Instruction {
        Instruction::Store(self)
    }
}

impl Allocate for StoreBucket {
    fn allocate(self) -> InstructionPointer {
        InstructionPointer::new(self.into_instruction())
    }
}

impl ObtainMeta for StoreBucket {
    fn get_line(&self) -> usize {
        self.line
    }
    fn get_message_id(&self) -> usize {
        self.message_id
    }
}

impl ToString for StoreBucket {
    fn to_string(&self) -> String {
        let line = self.line.to_string();
        let template_id = self.message_id.to_string();
        let dest_type = self.dest_address_type.to_string();
        let dest = self.dest.to_string();
        let src = self.src.to_string();
        format!(
            "STORE(line:{},template_id:{},dest_type:{},dest:{},src:{})",
            line, template_id, dest_type, dest, src
        )
    }
}

impl WriteWasm for StoreBucket {
    fn produce_wasm(&self, producer: &WASMProducer) -> Vec<String> {
        use code_producers::wasm_elements::wasm_code_generator::*;
        let mut instructions = vec![];
        if self.context.size == 0 {
            return vec![];
        }
        if producer.needs_comments() {
            instructions.push(format!(";; store bucket. Line {}", self.line)); //.to_string()
        }
        let mut my_template_header = Option::<String>::None;
        if producer.needs_comments() {
            instructions.push(";; getting dest".to_string());
        }
        match &self.dest {
            LocationRule::Indexed {
                location,
                template_header,
            } => {
                let mut instructions_dest = location.produce_wasm(producer);
                instructions.append(&mut instructions_dest);
                let size = producer.get_size_32_bits_in_memory() * 4;
                instructions.push(set_constant(&size.to_string()));
                instructions.push(mul32());
                match &self.dest_address_type {
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
                match &self.dest_address_type {
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
                                                                                            //now we have first the offset, and then the all size dimensions but the last one
                        if indexes.len() <= 1 {
                            instructions.push(load32(None)); // get signal offset (it is already the actual one in memory);
                            if indexes.len() == 1 {
                                let mut instructions_idx0 = indexes[0].produce_wasm(producer);
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
                                let mut instructions_idxi = indexes[i].produce_wasm(producer);
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
        if producer.needs_comments() {
            instructions.push(";; getting src".to_string());
        }
        if self.context.size > 1 {
            instructions.push(set_local(producer.get_store_aux_1_tag()));
        }
        let mut instructions_src = self.src.produce_wasm(producer);
        instructions.append(&mut instructions_src);
        if self.context.size == 1 {
            instructions.push(call("$Fr_copy"));
        } else {
            instructions.push(set_local(producer.get_store_aux_2_tag()));
            instructions.push(set_constant(&self.context.size.to_string()));
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
        match &self.dest_address_type {
            AddressType::SubcmpSignal { .. } => {
                // if subcomponent input check if run needed
                if producer.needs_comments() {
                    instructions.push(";; decrease counter".to_string()); // by self.context.size
                }
                instructions.push(get_local(producer.get_sub_cmp_tag())); // to update input signal counter
                instructions.push(get_local(producer.get_sub_cmp_tag())); // to read input signal counter
                instructions.push(load32(Some(
                    &producer
                        .get_input_counter_address_in_component()
                        .to_string(),
                ))); //remaining inputs to be set
                instructions.push(set_constant(&self.context.size.to_string()));
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
                match &self.dest {
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
        if producer.needs_comments() {
            instructions.push(";; end of store bucket".to_string());
        }
        instructions
    }
}

impl WriteRust for StoreBucket {
    fn produce_rust(&self, producer: &RustProducer, parallel: Option<bool>) -> (Vec<String>, String) {
        use rust_code_generator::*;
        let mut prologue = vec![];
        let cmp_index_ref = "cmp_index_ref".to_string();
        let aux_dest_index = "aux_dest_index".to_string();
        if let AddressType::SubcmpSignal { cmp_address, .. } = &self.dest_address_type {
            let (mut cmp_prologue, cmp_index) = cmp_address.produce_rust(producer, parallel);
            prologue.append(&mut cmp_prologue);
            prologue.push(format!("{{"));
            prologue.push(format!("let cmp_index_ref = {};", cmp_index));
        }
        let ((mut dest_prologue, dest_index), my_template_header) = match &self.dest {
            LocationRule::Indexed { location, template_header } => {
                (location.produce_rust(producer, parallel), template_header.clone())
            }
            LocationRule::Mapped { signal_code, indexes } => {
                // if Mapped must be subcomponent signal
                let mut map_prologue = vec![];
                let mut map_access = format!(
                    "ctx.circuit.template_to_io_signal[&ctx.components[subcomponents[cmp_index_ref]].template_id][{}].offset",
                    signal_code
                );
                if indexes.len() > 0 {
                    map_prologue.push(format!("{{"));
                    map_prologue.push(format!("let mut map_index_aux = vec![0usize; {}];", indexes.len().to_string()));
                    let (mut index_code_0, mut map_index) = indexes[0].produce_rust(producer, parallel);
                    map_prologue.append(&mut index_code_0);
                    map_prologue.push(format!("map_index_aux[0] = {};", map_index));
                    map_index = format!("map_index_aux[0]");
                    for i in 1..indexes.len() {
                        let (mut index_code, index_exp) = indexes[i].produce_rust(producer, parallel);
                        map_prologue.append(&mut index_code);
                        map_prologue.push(format!("map_index_aux[{}] = {};", i.to_string(), index_exp));
                        map_index = format!(
                            "map_index_aux[0] * ctx.circuit.template_to_io_signal[&ctx.components[subcomponents[cmp_index_ref]][{}].lengths[{}] + map_index_aux[{}]",
                            signal_code,
                            i - 1,
                            i
                        );
                    }
                    map_access = format!("{}+{}", map_access, map_index);
                }
                ((map_prologue, map_access), Some("ctx.components[subcomponents[cmp_index_ref]].template_id".into()))
            }
        };

        prologue.append(&mut dest_prologue);
        // Build dest
        let dest = match &self.dest_address_type {
            AddressType::Variable => {
                format!("lvar[{}]", dest_index.clone())
            }
            AddressType::Signal => {
                format!("ctx.signals[signal_start + {}]", dest_index.clone())
            }
            AddressType::SubcmpSignal { .. } => {
                let sub_cmp_start = format!("ctx.components[subcomponents[{}]].signal_start", cmp_index_ref);
                format!("ctx.signals[{} + {}]", sub_cmp_start, dest_index.clone())
            }
        };
        // keep dest_index in an auxiliar if parallel and out put
        if let AddressType::Signal = &self.dest_address_type {
            if parallel.unwrap() && self.dest_is_output {
                prologue.push(format!("{{"));
                prologue.push(format!("uint {} = {};", aux_dest_index, dest_index.clone()));
            }
        }
        // store src in dest
        prologue.push(format!("{{"));
        // Load src
        prologue.push(format!("// load src"));
        let (mut src_prologue, src) = self.src.produce_rust(producer, parallel);
        prologue.append(&mut src_prologue);
        prologue.push(format!("// end load src"));
        std::mem::drop(src_prologue);
        if self.context.size > 1 {
            let copy_arguments = vec![dest, src, self.context.size.to_string()];
            prologue.push(format!("{};", build_call("Fr_copyn".to_string(), copy_arguments)));
            if let AddressType::Signal = &self.dest_address_type {
                if parallel.unwrap() && self.dest_is_output {
                    unimplemented!()
                }
            }
        } else {
            prologue.push(format!("{} = {};", dest, src));
            if let AddressType::Signal = &self.dest_address_type {
                if parallel.unwrap() && self.dest_is_output {
                    unimplemented!()
                }
            }
        }
        prologue.push(format!("}}"));
        match &self.dest_address_type {
            AddressType::SubcmpSignal { uniform_parallel_value, input_information, .. } => {
                // if subcomponent input check if run needed
                let input_counter = format!("ctx.components[subcomponents[cmp_index_ref]].input_counter");
                // let sub_cmp_counter_decrease = format!("{} -= {}", input_counter, self.context.size);
                if let InputInformation::Input { status } = input_information {
                    if let StatusInput::NoLast = status {
                        // no need to run subcomponent
                        prologue.push("// no need to run sub component".to_string());
                        prologue.push(format!("{} -= {};", input_counter, self.context.size));
                        prologue.push(format!("assert!({} != 0);", input_counter));
                    } else {
                        let sub_cmp_call_arguments =
                            vec!["subcomponents[cmp_index_ref]".to_string(), "ctx".to_string()];
                        // to create the call instruction we need to consider the cases of parallel/not parallel/ known only at execution
                        if uniform_parallel_value.is_some() {
                            let mut call_instructions = if uniform_parallel_value.unwrap() {
                                unimplemented!();
                            }
                            // Case not parallel
                            else {
                                let sub_cmp_call_name = if let LocationRule::Indexed { .. } = &self.dest {
                                    format!("{}_run", my_template_header.unwrap())
                                } else {
                                    format!("(ctx.function_table[{}])", my_template_header.unwrap())
                                };
                                vec![format!("{};", build_call(sub_cmp_call_name, sub_cmp_call_arguments))]
                            };
                            if let StatusInput::Unknown = status {
                                prologue.push("// run sub component if needed".to_string());
                                prologue.push(format!("{} -= {};", input_counter, self.context.size));
                                let cond = format!("{} == 0", input_counter);
                                let else_instructions = vec![];
                                let mut conditional = build_conditional(cond, call_instructions, else_instructions);
                                prologue.append(&mut conditional);
                            } else {
                                prologue.push("// need to run sub component".to_string());
                                prologue.push(format!("{} -= {};", input_counter, self.context.size));
                                prologue.push(format!("assert!({} == 0);", input_counter));
                                prologue.append(&mut call_instructions);
                            }
                        }
                        // Case we only know if it is parallel at execution
                        else {
                            // case not parallel
                            let sub_cmp_call_name = if let LocationRule::Indexed { .. } = &self.dest {
                                format!("{}_run", my_template_header.unwrap())
                            } else {
                                format!("(*{}[{}])", function_table(), my_template_header.unwrap())
                            };
                            let mut call_instructions =
                                vec![format!("{};", build_call(sub_cmp_call_name, sub_cmp_call_arguments))];

                            if let StatusInput::Unknown = status {
                                prologue.push("// run sub component if needed".to_string());
                                prologue.push(format!("{} -= {};", input_counter, self.context.size));
                                let cond = format!("{} == 0", input_counter);
                                let else_instructions = vec![];
                                let mut conditional = build_conditional(cond, call_instructions, else_instructions);
                                prologue.append(&mut conditional);
                            } else {
                                prologue.push("// need to run sub component".to_string());
                                prologue.push(format!("{} -= {};", input_counter, self.context.size));
                                prologue.push(format!("assert!({} == 0);", input_counter));
                                prologue.append(&mut call_instructions);
                            }
                        }
                    }
                } else {
                    assert!(false);
                }
            }
            _ => (),
        }
        if let AddressType::SubcmpSignal { .. } = &self.dest_address_type {
            prologue.push(format!("}}"));
        }
        if let LocationRule::Mapped { indexes, .. } = &self.dest {
            if indexes.len() > 0 {
                prologue.push(format!("}}"));
            }
        }

        (prologue, "".to_string())
    }
}
