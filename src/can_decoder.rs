use crate::canlog_reader::CanFrame;
use can_dbc::DBC;
use rand::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::hint::black_box;
use std::io::{self, Read};
use std::time::Instant;

pub struct SignalsMap {
    pub names: Vec<String>,
    pub values: Vec<f64>,
}

impl SignalsMap {
    pub fn new(signal_names: &[&str], values: &[f64]) -> SignalsMap {
        let owned_strings = signal_names.iter().map(|s| (*s).to_owned()).collect();
        SignalsMap {
            names: owned_strings,
            values: values.to_owned(),
        }
    }
}

impl fmt::Display for SignalsMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\n")?;
        for (name, value) in self.names.iter().zip(self.values.iter()) {
            write!(f, "{}: {},\n", name, value)?;
        }
        write!(f, "}}")?;
        return Ok(());
    }
}

pub struct DecodedCanMessage {
    pub id: u32,
    pub name: String,
    pub signals: Vec<String>,
    pub values: Vec<f64>,
    pub units: Vec<String>,
}

impl fmt::Display for DecodedCanMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} : {:#X} ", self.name, self.id)?;
        write!(f, "{{\n")?;
        //Alternate form prints horizontally
        if f.alternate() {
            for signal_name in self.signals.iter() {
                write!(f, "{signal_name} \t")?;
            }
            write!(f, "\n")?;
            for value in self.values.iter() {
                write!(f, "{value} \t")?;
            }
        }
        //Standard form prints vertically
        else {
            for (signal_name, value) in self.signals.iter().zip(self.values.iter()) {
                write!(f, "{}: {},\n", signal_name, value)?;
            }
        }

        write!(f, "}}\n")?;
        return Ok(());
    }
}

/// Decode all the signal values from a given message
pub fn decode_message(can_frame: &CanFrame, message_spec: &can_dbc::Message) -> DecodedCanMessage {
    let mut values = Vec::with_capacity(message_spec.signals().len());
    let mut names = Vec::with_capacity(message_spec.signals().len());
    let mut units = Vec::with_capacity(message_spec.signals().len());
    for signal_spec in message_spec.signals() {
        names.push(signal_spec.name().clone());
        values.push(decode_signal(&can_frame, &signal_spec));
        units.push(signal_spec.unit().to_owned());
    }

    return DecodedCanMessage {
        id: can_frame.id,
        name: message_spec.message_name().clone(),
        signals: names,
        values: values,
        units: units,
    };
}

/*
pub fn can_decoder(can_msg: CanFrame, message_format: CanMessageFormat) -> SignalsMap {
    return SignalsMap::new(&["empty"], &[1.0]);
}*/

/// Extract the signal value from data of a CanFrame, based on specification of signal_spec
pub fn decode_signal(can_frame: &CanFrame, signal_spec: &can_dbc::Signal) -> f64 {
    /*println!(
        "--SIGNAL NAME: {:?}, SIGNAL UNIT {:?}---",
        signal_spec.name(),
        signal_spec.unit()
    );*/
    let v = signal_spec.value_type();
    let start_bit = signal_spec.start_bit;
    let mut byte_index = signal_spec.start_bit / 8;
    let mut bit_index = (signal_spec.start_bit % 8) as i32;
    let mut len = signal_spec.signal_size;
    let mut result: u64 = 0;
    if signal_spec.byte_order() == &can_dbc::ByteOrder::BigEndian {
        while len != 0 {
            //println!("byte index {byte_index}");
            while len != 0 && bit_index >= 0 {
                len -= 1;
                result = result << 1;
                let bit_val = ((can_frame.data[byte_index as usize] >> bit_index) & 0x01) as u64;
                result |= bit_val;
                //println!("bit_index {bit_index}, bit_val {bit_val}, result {result}");
                bit_index -= 1;
            }
            byte_index += 1;
            bit_index = 7;
        }
    } else {
        while len != 0 {
            //println!("byte index {byte_index}");
            //bit_index = 7 - bit_index;
            //let mut i = 0;
            //result = result << 8;
            while len != 0 && bit_index <= 7 {
                len -= 1;
                result = result >> 1;
                let bit_val = ((can_frame.data[byte_index as usize] >> bit_index) & 0x01) as u64;
                result |= bit_val << (signal_spec.signal_size - 1);
                //result |= bit_val << i;
                //i += 1;
                //println!("bit_index {bit_index}, bit_val {bit_val}, result {result}");
                bit_index += 1;
            }

            byte_index += 1;
            bit_index = 0;
        }
    }
    //return result as f64;

    //conversion from raw_signal to real value
    compute_signal_value(result, signal_spec)
}

/// Compute the final value of a CAN signal using the formula
/// final_value = decoded_signal_value * factor + offset
fn compute_signal_value(decoded_value: u64, signal_spec: &can_dbc::Signal) -> f64 {
    //conversion from raw_signal to real value
    let final_value = match signal_spec.value_type() {
        //Sign extend if the value is signed
        can_dbc::ValueType::Signed => {
            let shift_len = 64 - signal_spec.signal_size;
            //Sign extend operation: shift left to place MSB into top of u64, shift right to get sign extension.
            let sign_extended = ((decoded_value as i64) << shift_len) >> shift_len;
            //println!("shift len {shift_len} sign_extended {sign_extended}");
            sign_extended as f64
        }
        can_dbc::ValueType::Unsigned => decoded_value as f64,
    };
    return final_value * signal_spec.factor() + signal_spec.offset();
}

/// Extract the signal value from data of a CanFrame, based on specification of signal_spec
/// Read 1 byte at a time when decoding
pub fn decode_signal_by_bytes(can_frame: &CanFrame, signal_spec: &can_dbc::Signal) -> f64 {
    let v: &can_dbc::ValueType = signal_spec.value_type();
    let start_bit = signal_spec.start_bit;
    let mut byte_index = signal_spec.start_bit / 8;
    let mut bit_index = (signal_spec.start_bit % 8) as i32;
    let mut len = signal_spec.signal_size;
    let mut result: u64 = 0;
    let masks = [
        0b1,
        0b11,
        0b111,
        0b1111,
        0b1_1111,
        0b11_1111,
        0b111_1111,
        0b1111_1111,
    ];
    //println!("{:?}", masks);
    if signal_spec.byte_order() == &can_dbc::ByteOrder::BigEndian {
        while len > 0 {
            let bit_end = if (bit_index + 1 - len as i32) < 0 {
                0
            } else {
                bit_index + 1 - len as i32
            };
            //println!("Bit index {bit_index}, bit_end {bit_end}");

            let incoming_bit_len = bit_index + 1 - bit_end;
            let mask_index = incoming_bit_len - 1;
            result = result << incoming_bit_len;
            let byte_val = ((can_frame.data[byte_index as usize] >> bit_end)
                & masks[mask_index as usize]) as u64;
            result |= byte_val;
            //println!("incoming bit len {incoming_bit_len}, byte_val {byte_val}, len left {len}");
            len -= incoming_bit_len as u64;
            byte_index += 1;
            bit_index = 7;
        }
    } else {
        while len != 0 {
            let bit_end = if (bit_index + (len as i32) - 1) >= 8 {
                7
            } else {
                bit_index + (len as i32) - 1
            };
            //println!("Bit index {bit_index}, bit_end {bit_end}");

            let incoming_bit_len = bit_end - bit_index + 1;
            let mask_index = incoming_bit_len - 1;
            //result = result << incoming_bit_len;
            let byte_val = ((can_frame.data[byte_index as usize] >> bit_index)
                & masks[mask_index as usize]) as u64;
            result |= byte_val << (signal_spec.signal_size - len);
            //println!("incoming bit len {incoming_bit_len}, byte_val {byte_val}, len left {len}");
            len -= incoming_bit_len as u64;
            byte_index += 1;
            bit_index = 0;
        }
    }
    //return result as f64;
    compute_signal_value(result, signal_spec)
}

pub fn load_dbc(dbc_path: &str) -> io::Result<can_dbc::DBC> {
    let mut dbc_file = File::open(&dbc_path)?;
    let mut buffer = Vec::new();
    dbc_file.read_to_end(&mut buffer)?;

    /*match can_dbc::DBC::from_slice(&buffer) {
        Ok(can_dbc) => Ok(can_dbc),
        Err(e) => io::Error(e.kind()),
    }*/
    let maybe_dbc = can_dbc::DBC::from_slice(&buffer);
    match maybe_dbc {
        Ok(dbc) => Ok(dbc),
        _ => Err(io::Error::new(io::ErrorKind::Other,"Error loading dbc")),
    }
}

/// Retreive specification of  the message as read from the CAN DBC
pub fn get_message_spec<'a>(
    dbc: &'a can_dbc::DBC,
    message_name: &str,
) -> Option<&'a can_dbc::Message> {
    let msg = dbc
        .messages()
        .iter()
        .find(|m| m.message_name() == message_name);
    return msg;
}

/// Retrieve the specification of the signal as read from the CAN DBC
pub fn get_signal_spec<'a>(
    message_spec: &'a can_dbc::Message,
    signal_name: &str,
) -> Option<&'a can_dbc::Signal> {
    let signal = message_spec
        .signals()
        .iter()
        .find(|s| s.name() == signal_name);
    return signal;
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
    use can_dbc::Message;

    use crate::canlog_reader;

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
    #[test]
    fn test_load_dbc() {
        let dbc = load_dbc("motohawk.dbc").unwrap();
        let dbc = load_dbc("signed.dbc").unwrap();
        //let dbc = load_dbc("abs.dbc").unwrap();
        for message in dbc.messages() {
            for s in message.signals() {
                println!("{:?}", s);
            }
            //println!("{:?}", message);
        }
    }
    #[test]
    fn signal_decode_signed_dbc() {
        //s3big = 0b011
        /*
           Golden Sample using Cantools
           echo "(0.0) vcan0 00A#11223344FF667788" | python3 -m cantools decode signed.dbc
           (0.0) vcan0 00A#11223344FF667788 ::
           Message378910(
               s7: 8,
               s8big: -111,
               s9: 25,
               s8: -47,
               s3big: -1,
               s3: -1,
               s10big: 239,
               s7big: 8
           )
        */
        let s3big_expected = -1.0;
        let s3_expected = -1.0;
        let line = "(0.0) vcan0 00A#11223344FF667788";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = load_dbc("signed.dbc").unwrap();

        let msg = get_message_spec(&dbc, "Message378910").unwrap();
        let signal = get_signal_spec(&msg, "s3big").unwrap();

        let value = decode_signal(&frame, signal);
        let value2 = decode_signal_by_bytes(&frame, signal);
        println!("{:?}", frame);
        assert_eq!(value, s3big_expected);
        assert_eq!(value2, s3big_expected);
        //s3 (little endian) = 0b011 -> 0b110
        let signal = msg
            .signals()
            .iter()
            .find(|s| s.name() == "s3")
            .expect("could not find signal");
        let t1 = Instant::now();
        for i in 0..1000 {
            let value = decode_signal(&frame, signal);
            assert_eq!(value, s3_expected);
        }
        let bit_shift_time = t1.elapsed().as_micros();

        let t2 = Instant::now();
        for i in 0..1000 {
            let value2 = decode_signal_by_bytes(&frame, signal);
            //assert_eq!(value2, 6.0);
            assert_eq!(value2, s3_expected);
        }
        let byte_shift_time = t2.elapsed().as_micros();
        println!("Bit shifting time: {bit_shift_time}");
        println!("Byte shifting time: {byte_shift_time}");

        //assert_eq!(value, 6.0);
        //assert_eq!(value2, 6.0);

        //s64
        /*
        echo "(0.0) vcan0 002#11223344FF667788" | python3 -m cantools decode signed.dbc
        (0.0) vcan0 002#11223344FF667788 ::
        Message64(
            s64: -8613302515775888879
        )*/
        let s64_expected = -8613302515775888879.0;
        let msg = get_message_spec(&dbc, "Message64").unwrap();
        let signal = get_signal_spec(&msg, "s64").unwrap();

        let t3 = Instant::now();
        let mut running_sum = 0.0;
        for i in 0..10000 {
            let value2 = decode_signal_by_bytes(&frame, signal);
            //println!("get signal by bytes value {value2}");
            running_sum += value2;
            assert_eq!(value2, s64_expected);
        }
        println!("running sum {running_sum}");
        let byte_shift_time = t3.elapsed().as_micros();

        let t4 = Instant::now();
        for i in 0..10000 {
            let value = decode_signal(&frame, signal);
            //println!("get signal value {value}");
            running_sum += value;
            assert_eq!(value, s64_expected);
        }
        let bit_shift_time = t4.elapsed().as_micros();
        println!("running sum 2 {running_sum}");
        println!("Bit shifting time: {bit_shift_time}");
        println!("Byte shifting time: {byte_shift_time}");
    }
    #[test]
    fn motohawk_decode_signal() {
        //Temperature = 0b001110111011 = 955
        /* golden sample using cantools
        echo "(0.0) vcan0 1F0#A5B6D90000000000" | python3 -m cantools decode motohawk.dbc
        (0.0) vcan0 1F0#A5B6D90000000000 ::
        ExampleMessage(
            Enable: Enabled,
            AverageRadius: 1.8 m,
            Temperature: 244.14 degK
        )
         */
        let line = "(0.0) vcan0 1F0#A5B6D90000000000";
        /*let expected_signals = HashMap::from([
            ("Enable", 1.0),
            ("AverageRadius", 1.8),
            ("Temperature", 244.14),
        ]);*/
        let expected_enable = true;
        let expected_average_radius = 1.8;
        let expected_temperature = 244.14;

        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = load_dbc("motohawk.dbc").unwrap();
        let msg = get_message_spec(&dbc, "ExampleMessage").unwrap();
        let signal = get_signal_spec(&msg, "Temperature").unwrap();

        let time2 = Instant::now();
        for _i in 0..1000 {
            let value2 = decode_signal_by_bytes(&frame, signal);
            assert_eq!(value2, expected_temperature);
        }
        let byte_shifting_time = time2.elapsed().as_micros();

        let time = Instant::now();
        for _i in 0..1000 {
            let value = decode_signal(&frame, signal);
            assert_eq!(value, expected_temperature);
        }
        let bit_shifting_time = time.elapsed().as_micros();
        println!("Bit shifting time: {bit_shifting_time}");
        println!("Byte shifting time: {byte_shifting_time}");
        println!("{:?}", frame);
        //assert_eq!(value, 955.0);
        //assert_eq!(value2, 955.0);
    }
    #[test]
    fn test_decode_msg_and_parse() {
        let line = "(0.0) vcan0 1F0#A5B6D90000000000";
        /*let expected_signals = HashMap::from([
            ("Enable", 1.0),
            ("AverageRadius", 1.8),
            ("Temperature", 244.14),
        ]);*/
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = load_dbc("motohawk.dbc").unwrap();
        let msg_spec = get_message_spec(&dbc, "ExampleMessage").unwrap();
        let msg = decode_message(&frame, &msg_spec);

        println!("{msg}");
    }
}

/*
jacob@jacob-ubuntu:~/rust_projects/rocketcan$ echo "(0.0) vcan0 1F0#0077733445566778" | python3 -m cantools decode motohawk.dbc
(0.0) vcan0 1F0#0077733445566778 ::
ExampleMessage(
    Enable: Disabled,
    AverageRadius: 0.0 m,
    Temperature: 259.55 degK
)
jacob@jacob-ubuntu:~/rust_projects/rocketcan$ echo "(0.0) vcan0 002#11223344FF667788" | python3 -m cantools decode signed.dbc
(0.0) vcan0 002#11223344FF667788 ::
Message64(
    s64: -8613302515775888879
)
jacob@jacob-ubuntu:~/rust_projects/rocketcan$ echo "(0.0) vcan0 003#8000000000000000" | python3 -m cantools decode signed.dbc
(0.0) vcan0 003#8000000000000000 ::
Message64big(
    s64big: -9223372036854775808
)
jacob@jacob-ubuntu:~/rust_projects/rocketcan$ echo "(0.0) vcan0 1F0#0077733445566778" | python3 -m cantools decode motohawk.dbc
(0.0) vcan0 1F0#0077733445566778 ::
ExampleMessage(
    Enable: Disabled,
    AverageRadius: 0.0 m,
    Temperature: 259.55 degK
)
jacob@jacob-ubuntu:~/rust_projects/rocketcan$ echo "(0.0) vcan0 00A#11223344FF667788" | python3 -m cantools decode signed.dbc
(0.0) vcan0 00A#11223344FF667788 ::
Message378910(
    s7: 8,
    s8big: -111,
    s9: 25,
    s8: -47,
    s3big: -1,
    s3: -1,
    s10big: 239,
    s7big: 8

echo "(0.0) vcan0 00A#11223244FF017788" | python3 -m cantools decode signed.dbc
Message378910(
    s7: 8,
    s8big: -111,
    s9: 25,
    s8: -47,
    s3big: -1,
    s3: -1,
    s10big: 239,
    s7big: 8
)

DUMPING TOOLS:
python3 -m cantools dump signed.db

*/
