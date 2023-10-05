use ff::{PrimeField, PrimeFieldBits};
use lz_fnv::Fnv1a;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::witness_generator::{
    get_size_of_input_hashmap, get_main_input_signal_start, run, get_total_signal_no, get_main_input_signal_no,
    get_number_of_components,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashSignalInfo {
    pub hash: u64,
    pub signal_id: usize,
    pub signal_size: usize,
}

impl HashSignalInfo {
    pub fn new(hash: u64, signal_id: usize, signal_size: usize) -> Self {
        HashSignalInfo { hash, signal_id, signal_size }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IODef {
    pub code: usize,
    pub offset: usize,
    pub lengths: Vec<usize>,
}

// It is an array that contains (name, start position, size)
pub type InputList = Vec<(String, usize, usize)>;
pub type TemplateList = Vec<String>;
pub struct InfoParallel {
    pub name: String,
    pub is_parallel: bool,
    pub is_not_parallel: bool,
}
pub type TemplateListParallel = Vec<InfoParallel>;
pub type SignalList = Vec<usize>;
pub type InputOutputList = Vec<IODef>;
pub type TemplateInstanceIOMap = BTreeMap<usize, InputOutputList>;
pub type MessageList = Vec<String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    pub map: Vec<HashSignalInfo>,
    pub signal_list: SignalList,
    pub constants: Vec<String>,
    pub iomap: TemplateInstanceIOMap,
}

pub fn hasher(value: &str) -> u64 {
    use lz_fnv::FnvHasher;
    let mut fnv_hasher: Fnv1a<u64> = Fnv1a::with_key(14695981039346656037);
    fnv_hasher.write(value.as_bytes());
    fnv_hasher.finish()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircomCircuit<F: PrimeField> {
    pub input_hash_map: Vec<HashSignalInfo>,
    pub witness_to_signal: SignalList,
    pub constants: Vec<F>,
    pub template_to_io_signal: TemplateInstanceIOMap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircomComponent {
    pub template_id: usize,
    pub signal_start: usize,
    pub input_counter: usize,
    pub template_name: String,
    pub component_name: String,
    pub father_id: usize,
    pub subcomponents: Vec<usize>,
}

impl CircomComponent {
    pub fn blank() -> Self {
        CircomComponent {
            template_id: 0,
            signal_start: 0,
            input_counter: 0,
            template_name: "none".into(),
            component_name: "none".into(),
            father_id: 0,
            subcomponents: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircomWitness<F: PrimeField> {
    pub input_counter: usize,
    assigned_inputs: Vec<bool>,
    pub signals: Vec<F>,
    pub components: Vec<CircomComponent>,
    pub circuit: CircomCircuit<F>,
    #[serde(skip)]
    pub function_table: Vec<for<'a> fn(usize, &'a mut CircomWitness<F>) -> ()>,
}

impl<F: PrimeFieldBits> CircomWitness<F> {
    pub fn new(circuit: CircomCircuit<F>) -> Self {
        let input_counter = get_main_input_signal_no();
        let assigned_inputs = vec![false; input_counter];
        let mut signals = vec![F::ZERO; get_total_signal_no()];
        signals[0] = F::ONE;
        let components = vec![CircomComponent::blank(); get_number_of_components()];
        let function_table = Vec::new();

        CircomWitness { input_counter, assigned_inputs, signals, components, circuit, function_table }
    }

    pub fn try_run_circuit(&mut self) {
        if self.input_counter == 0 {
            run(self);
        }
    }

    pub fn set_input_signal(&mut self, h: u64, i: usize, val: &F) {
        if self.input_counter == 0 {
            panic!("no more input signals to be assigned");
        }
        let pos = self.get_input_signal_hash_position(h);
        if i >= self.circuit.input_hash_map[pos].signal_size {
            panic!("input signal array access exceeds the size");
        }
        let si = self.circuit.input_hash_map[pos].signal_id + i;
        if self.assigned_inputs[si - get_main_input_signal_start()] {
            panic!("signal assigned twice {si}");
        }
        self.signals[si] = *val;
        self.assigned_inputs[si - get_main_input_signal_start()] = true;
        self.input_counter -= 1;
        self.try_run_circuit();
    }

    pub fn get_witness(&self, idx: usize) -> F {
        self.signals[self.circuit.witness_to_signal[idx]]
    }

    pub fn get_input_signal_size(&self, h: u64) -> usize {
        let pos = self.get_input_signal_hash_position(h);
        self.circuit.input_hash_map[pos].signal_size
    }

    pub fn get_input_signal_hash_position(&self, h: u64) -> usize {
        let n = get_size_of_input_hashmap();
        let mut pos = (h % (n as u64)) as usize;
        if self.circuit.input_hash_map[pos].hash == h {
            let init_pos = pos;
            pos += 1;
            while pos != init_pos {
                if self.circuit.input_hash_map[pos].hash == h {
                    return pos;
                }
                if self.circuit.input_hash_map[pos].hash == h {
                    panic!("signal not found");
                }
                pos = (pos + 1) % n;
            }
            panic!("signal not found");
        }
        pos
    }
}

/// A representation of circom numeric types, either a normal field element or a u64.
/// The types are meant to be separate, with u64s representing the compile time constants
/// in template parameters and loops, and scalars representing signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Num<F: PrimeField> {
    /// A scalar field element.
    Scalar(F),
    /// A compile time constant, like a template parameter or loop constant.
    U64(u64),
}

impl<F: PrimeField> Num<F> {
    pub fn get_u64(&self) -> u64 {
        match self {
            Num::Scalar(_) => panic!(),
            Num::U64(x) => *x,
        }
    }
}
