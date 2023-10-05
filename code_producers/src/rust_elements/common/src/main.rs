#![allow(non_snake_case)]

use std::{
    fs::File,
    io::{Error, BufReader, Write},
};

use byteorder::{WriteBytesExt, LittleEndian};
use circom::{Data, CircomCircuit, hasher, CircomWitness};
use ff::PrimeFieldBits;
use pasta_curves::pallas;
use serde_json::Value;
use witness_generator::{get_size_of_witness, get_main_input_signal_no};

pub mod circom;
pub mod field;
pub mod witness_generator;

pub fn load_circuit<F: PrimeFieldBits>(dat_file_name: &str) -> Result<CircomCircuit<F>, Error> {
    let file = File::open(dat_file_name)?;
    let reader = BufReader::new(file);
    let data: Data = serde_json::from_reader(reader)?;
    let constants = data
        .constants
        .iter()
        .map(|f| F::from_str_vartime(f))
        .collect::<Option<Vec<F>>>()
        .expect("failed to decode constants");
    let circom_circuit: CircomCircuit<F> = CircomCircuit {
        input_hash_map: data.map,
        witness_to_signal: data.signal_list,
        constants,
        template_to_io_signal: data.iomap,
    };
    Ok(circom_circuit)
}

fn load_field_elements<F: PrimeFieldBits>(v: &Value) -> Vec<F> {
    match v {
        Value::String(x) => vec![F::from_str_vartime(x).expect("not a field element")],
        Value::Array(xs) => {
            // how to write better rust code?
            xs.iter().map(|x| {
                if let Value::String(x) = x {
                    F::from_str_vartime(x).expect("not a field element")
                } else {
                    panic!("not a field element")
                }
            })
        }
        .collect(),
        _ => panic!("malformed input file"), // TODO: actual circom errors
    }
}

pub fn load_input_json<F: PrimeFieldBits>(input_file_name: &str, ctx: &mut CircomWitness<F>) -> Result<(), Error> {
    let file = File::open(input_file_name)?;
    let reader = BufReader::new(file);
    let inputs = if let Value::Object(map) = serde_json::from_reader(reader)? {
        map
    } else {
        panic!("malformed input file") // TODO: actual circom errors
    };

    for (key, value) in inputs.iter() {
        let h = hasher(key) % 256;
        let v = load_field_elements::<F>(value);

        if v.len() < ctx.get_input_signal_size(h) {
            panic!("error loading signal {key}: not enough values");
        } else if v.len() > ctx.get_input_signal_size(h) {
            panic!("error loading signal {key}: too many values {}", ctx.get_input_signal_size(h));
        }

        for (i, val) in v.iter().enumerate() {
            ctx.set_input_signal(h, i, val);
        }
    }
    Ok(())
}

pub fn write_bin_witness<F: PrimeFieldBits>(wtns_file_name: &str, ctx: &mut CircomWitness<F>) -> Result<(), Error> {
    let mut file = File::create(wtns_file_name)?;
    let mut buffer: Vec<u8> = Vec::new();
    buffer.write(b"wtns")?;

    let version = 2_u32;
    buffer.write_u32::<LittleEndian>(version)?;

    let n_sections = 2_u32;
    buffer.write_u32::<LittleEndian>(n_sections)?;

    let id_section_1 = 1_u32;
    buffer.write_u32::<LittleEndian>(id_section_1)?;

    let n8 = std::mem::size_of::<F>();
    let id_section_length = 8 + n8;
    buffer.write_u64::<LittleEndian>(id_section_length as u64)?;
    buffer.write_u32::<LittleEndian>(n8 as u32)?;

    Ok(())
}

/// Usage: cargo run --release <circuit> <input.json> <output.wtns>
fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: <circuit> <input.json> <output.wtns>");
        return Ok(());
    }

    let datfile = format!("{}.dat", args[1]);
    let jsonfile = args[2].clone();
    let wtnsfile = args[3].clone();

    let circuit = load_circuit::<pallas::Base>(&datfile)?;
    let mut ctx = CircomWitness::new(circuit);

    load_input_json(&jsonfile, &mut ctx)?;

    if ctx.input_counter != 0 {
        let total = get_main_input_signal_no();
        panic!("Not all inputs have been set. Only {} out of {}.", total - ctx.input_counter, total);
    }

    let n_wtns = get_size_of_witness();

    for i in 0..n_wtns {
        let f = ctx.get_witness(i);
        println!("{:?}", f);
    }

    Ok(())
}
