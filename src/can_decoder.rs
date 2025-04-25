use crate::canlog_reader::CanFrame;
use can_dbc::DBC;
use rand::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::hint::black_box;
use std::io::{self, Read};
use std::time::Instant;

pub struct SignalsMap {
    names: Vec<String>,
    values: Vec<f32>,
}

impl SignalsMap {
    pub fn new(signal_names: &[&str], values: &[f32]) -> SignalsMap {
        let owned_strings = signal_names.iter().map(|s| (*s).to_owned()).collect();
        SignalsMap {
            names: owned_strings,
            values: values.to_owned(),
        }
    }
}

pub fn can_decoder(can_msg: CanFrame, message_format: CanMessageFormat) -> SignalsMap {
    return SignalsMap::new(&["empty"], &[1.0]);
}

pub fn load_dbc(dbc_path: &str) -> io::Result<can_dbc::DBC> {
    let mut dbc_file = File::open(&dbc_path)?;
    let mut buffer = Vec::new();
    dbc_file.read_to_end(&mut buffer)?;

    /*match can_dbc::DBC::from_slice(&buffer) {
        Ok(can_dbc) => Ok(can_dbc),
        Err(e) => io::Error(e.kind()),
    }*/
    Ok(can_dbc::DBC::from_slice(&buffer).unwrap())
}

//A slice of string slices
const SIGNAL_NAMES: &[&str] = &[
    "shrt",
    "a_medium_length_name",
    "a_really_really_long_signal_name",
    "asdfs",
    "torqueValueName",
    "steeringSignalName",
    "brakingValueEbc1",
    "ThrottleValueOpen",
    "Thisis123",
    "LastOne",
];
const SIGNAL_VALUES: [f32; 10] = [
    10.,
    100.432,
    87.5,
    26.,
    19.0,
    1003789.789,
    908.8979,
    12.3,
    456.,
    987.,
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn benchmark_hashmap() {
        //build hashmap
        let mut signals = HashMap::new();
        for (name, value) in SIGNAL_NAMES.iter().zip(SIGNAL_VALUES.iter()) {
            signals.insert(*name, value);
        }

        //Section 1: N access of random signals
        let mut rng = StdRng::seed_from_u64(10);
        const N: usize = 10000;
        let mut arr = [&0f32; N];
        let now = Instant::now();
        for i in 0..N {
            let random_index = rng.random_range(0..10);
            let signal_name = SIGNAL_NAMES[random_index];
            arr[i] = black_box(signals[black_box(signal_name)]);
        }
        let section_1_time = now.elapsed().as_micros();
        println!("Hashmap Section 1\n{:?}", arr);
        println!("------Hasmap Time 1: {}------", section_1_time);

        //Section 2: Same value N times
        let target_signal = SIGNAL_NAMES[9];
        let now_2 = Instant::now();
        for i in 0..N {
            arr[i] = black_box(signals[black_box(target_signal)]);
        }
        let section_2_time = now_2.elapsed().as_micros();
        println!("Hashmap Section 2\n{:?}", arr);
        println!("------Hashmap Time 2: {}------", section_2_time);
    }

    #[test]
    fn benchmark_vec() {
        //build vecs
        let mut vnames = Vec::with_capacity(SIGNAL_NAMES.len());
        let mut values = Vec::with_capacity(SIGNAL_NAMES.len());
        for i in 0..SIGNAL_VALUES.len() {
            vnames.push(SIGNAL_NAMES[i]);
            values.push(SIGNAL_VALUES[i]);
        }

        //Section 1: N access of random signals
        let mut rng = StdRng::seed_from_u64(10);
        const N: usize = 10000;
        let mut arr = [&0f32; N];
        let now = Instant::now();
        for i in 0..N {
            let random_index = rng.random_range(0..10);
            let signal_name = SIGNAL_NAMES[random_index];

            let index = black_box(vnames.iter().position(|name| *name == signal_name).unwrap());
            arr[i] = black_box(&values[black_box(index)]);
        }
        let section_1_time = now.elapsed().as_micros();
        println!("Vector Section 1\n{:?}", arr);
        println!("-----Vector Time 1: {}------", section_1_time);

        //Section 2: Same value N times
        let target_signal = SIGNAL_NAMES[9];
        let now_2 = Instant::now();

        for i in 0..N {
            let index = vnames
                .iter()
                .position(|name| *name == target_signal)
                .unwrap();
            arr[i] = &values[black_box(index)];
        }
        let section_2_time = now_2.elapsed().as_micros();
        println!("Vector Section 2\n{:?}", arr);
        println!("------Vector Time 2: {}------", section_2_time);
    }
}
