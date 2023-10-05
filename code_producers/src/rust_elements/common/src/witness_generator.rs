use ff::PrimeField;
use crate::{circom::CircomWitness, field};

pub fn get_main_input_signal_start() -> usize {
    2
}
pub fn get_main_input_signal_no() -> usize {
    2
}
pub fn get_total_signal_no() -> usize {
    4
}
pub fn get_number_of_components() -> usize {
    1
}
pub fn get_size_of_input_hashmap() -> usize {
    256
}
pub fn get_size_of_witness() -> usize {
    4
}
pub fn get_size_of_constants() -> usize {
    0
}
pub fn get_size_of_io_map() -> usize {
    0
}
pub fn Multiply_0_create<F: PrimeField>(
    soffset: usize,
    coffset: usize,
    ctx: &mut CircomWitness<F>,
    component_name: &str,
    component_father: usize,
) {
    ctx.components[coffset].template_id = 0;
    ctx.components[coffset].template_name = "Multiply".into();
    ctx.components[coffset].signal_start = soffset;
    ctx.components[coffset].input_counter = 2;
    ctx.components[coffset].component_name = component_name.into();
    ctx.components[coffset].father_id = component_father;
    ctx.components[coffset].subcomponents = vec![0; 0];
}

pub fn Multiply_0_run<F: PrimeField>(ctx_index: usize, ctx: &mut CircomWitness<F>) {
    let signals = &mut ctx.signals;
    let signal_start = ctx.components[ctx_index].signal_start;
    let template_name = &ctx.components[ctx_index].template_name;
    let component_name = &ctx.components[ctx_index].component_name;
    let father = ctx.components[ctx_index].father_id;
    let id = ctx_index;
    let subcomponents = &ctx.components[ctx_index].subcomponents;
    let constants = &ctx.circuit.circuit_constants;
    let mut expaux = vec![F::ZERO; 3];
    let mut lvar = vec![F::ZERO; 0];
    let subcomponent_aux: usize;
    let index_multiple_eq: usize;
    {
        // load src
        // start of load line 8 bucket { "type": "load", "line": 8, "template_id": 0, "address_type": "SIGNAL", "src": { "location_rule": "indexed", "location_msg": { "type": "value", "line": 0, "template_id": 0, "as": "U32", "op_number": 1, "value": 1 }, "header_msg": "none" } }
        // end of load line 8 with access signals[signal_start + 1]
        // start of load line 8 bucket { "type": "load", "line": 8, "template_id": 0, "address_type": "SIGNAL", "src": { "location_rule": "indexed", "location_msg": { "type": "value", "line": 0, "template_id": 0, "as": "U32", "op_number": 2, "value": 2 }, "header_msg": "none" } }
        // end of load line 8 with access signals[signal_start + 2]
        expaux[0] = signals[signal_start + 1] * signals[signal_start + 2]; // circom line 8
                                                                           // end load src
        field::copy(&mut signals[0], &expaux[0]);
    }
}

pub fn run<F: PrimeField>(ctx: &mut CircomWitness<F>) {

}