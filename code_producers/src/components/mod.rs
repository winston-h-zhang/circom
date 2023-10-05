use lz_fnv::Fnv1a;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

#[derive(Serialize, Deserialize)]
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
