use std::borrow::Borrow;
use std::fmt::Error;
use std::fmt::Write;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

#[derive(Debug)]
pub struct CanFrame {
    // Timestamp: Time the data was received (seconds)
    pub timestamp: f64,
    // CAN ID: 11-bit standard or 29-bit extended ID
    pub id: u32,
    // Data Length Code (DLC), 0 to 8 for CAN, 0 to 64 for CAN FD
    pub len: u8,
    // Payload data, can store up to 64 bytes for CAN FD, 8 bytes for standard CAN
    pub data: [u8; 64],
}
/*
(1436509052.249713) vcan0 044#2A366C2BBA
(1436509052.449847) vcan0 0F6#7ADFE07BD2
(1436509052.650004) vcan0 236#C3406B09F4C88036
(1436509052.850131) vcan0 6F1#98508676A32734
(1436509053.050284) vcan0 17F#C7
(1436509053.250417) vcan0 25B#6EAAC56C77D15E27
(1436509053.450557) vcan0 56E#46F02E79A2B28C7C
(1436509053.650713) vcan0 19E#6FE1CB7DE2218456
(1436509053.850870) vcan0 1A0#9C20407F96EA167B
(1436509054.051025) vcan0 6DE#68FF147114D1
*/

/// Turn ascii hex data into byte values
pub fn ascii_hex_to_bytes(hex_str: &str) -> [u8; 64] {
    let mut data_bytes = [0; 64];

    let mut index = 0;
    let mut i = 0;
    while i < hex_str.len() {
        data_bytes[index] = u8::from_str_radix(&hex_str[i..i + 2], 16)
            .expect(&format!("failed to parse data bytes {}", hex_str));
        index += 1;
        i += 2;
    }
    return data_bytes;
}

/// Parse a line in candump format
/// (1436509053.850870) vcan0 1A0#9C20407F96EA167B
/// ```
/// rocketcan::canlog_reader::parse_candump_line(" (1436509053.850870) vcan0 1A0#9C20407F96EA167B");
/// ```
pub fn parse_candump_line(line: &str) -> CanFrame {
    //Error in case parsing fails
    let error_msg = format!("Error parsing line: {}", line);

    let mut line_splits = line.split_whitespace();
    //Get timestamp
    let timestamp = line_splits.next().expect(&error_msg);
    let timestamp = &timestamp[1..timestamp.len() - 1];
    let timestamp = timestamp.parse::<f64>().expect(&error_msg);
    // CAN interface name
    let _interface_name = line_splits.next();
    //ID
    let id_and_data: Vec<_> = line_splits.next().expect(&error_msg).split('#').collect();
    let id = u32::from_str_radix(id_and_data[0], 16).expect(&error_msg);
    let ascii_data = id_and_data[1];
    let data = ascii_hex_to_bytes(id_and_data[1]);
    let data_len = (ascii_data.len() / 2) as u8;
    return CanFrame {
        timestamp: timestamp,
        id: id,
        len: data_len,
        data: data,
    };
}

/// Convert a CanFrame to an ascii candump line
pub fn frame_to_candump_line(frame: CanFrame) -> String {
    let mut s = format!("({}) vcan0 {:X}#", frame.timestamp, frame.id);
    for i in 0..frame.len as usize {
        write!(s, "{:02X}", frame.data[i]).unwrap();
    }
    return s;
}
pub struct CanLogReader<T>
where
    T: Iterator,
{
    iterable: T,
}
//Would have to implement trait for io::Lines
/*pub trait CanLogRead {
    fn to_canlog_reader(self) -> CanLogReader<Self>
    where
        Self: Sized,
    {
        CanLogReader { iterable: self }
    }
}

impl<T> CanLogRead for io::Lines<T> {}
*/
/*
impl<T> CanLogReader<T>
where
    T: Iterator<Item: std::borrow::Borrow<str>>,
    //T: IntoIterator<Item: std::borrow::Borrow<str>>,
    //I: Iterator<Item: std::borrow::Borrow<str>>,
{
    fn from_file(filename: &str) -> CanLogReader<io::Lines<BufReader<File>>> {
        let Ok(f) = File::open(filename) else {
            panic!("Unable to open file named {filename}");
        };
        let buf_reader = BufReader::new(f);
        let t = buf_reader.lines();
        CanLogReader { reader: t }
    }

    fn from_string<'a>(string: &'a String) -> CanLogReader<Vec<&str>> {
        let t = string.lines().collect();
        CanLogReader::<Vec<&str>> {
            reader: t,
            iter: None,
        }
    }
}*/

impl<T> Iterator for CanLogReader<T>
where
    T: Iterator<Item = std::io::Result<String>>,
    //T::Item: std::borrow::Borrow<str>,
{
    type Item = CanFrame;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(line) = self.iterable.next() {
            //println!("{}", line.unwrap());
            return Some(parse_candump_line(&line.unwrap()));
        }
        return None;
    }
}

type LinesFileBufReader = std::io::Lines<BufReader<File>>;
impl CanLogReader<LinesFileBufReader> {
    pub fn from_file(filename: &str) -> CanLogReader<LinesFileBufReader> {
        let Ok(f) = File::open(filename) else {
            panic!("Unable to open file named {filename}");
        };
        let buf_reader = BufReader::new(f);
        let lines = buf_reader.lines();
        let reader = CanLogReader { iterable: lines };
        return reader;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /*#[test]
    fn test_from_string() {
        let one_line_str = String::from("(1436509052.249713) vcan0 044#2A366C2BBA");
        let mut reader = CanLogReader::<Vec<&str>>::from_string(&one_line_str);
        println!("{:?}", reader.next());
    }*/
    #[test]
    fn test_file() {
        println!("HELLO WORLD----");
    }
    #[test]
    fn test_from_file() {
        let filename = "candump.log";
        let Ok(f) = File::open(filename) else {
            panic!("Unable to open file named {filename}");
        };
        let buf_reader = BufReader::new(f);
        let t = buf_reader.lines();
        //let next = t.next();
        //let mut cr = t.to_canlog_reader();
        let mut cr = CanLogReader { iterable: t };
        let cr = CanLogReader::from_file(filename);
        for can_frame in cr {
            println!("{:?}", can_frame);
        }
    }
    #[test]
    fn test_ascii_hex_data() {
        let expected = vec![1u8, 2u8, 17u8, 18u8, 10u8, 11u8];
        let result = ascii_hex_to_bytes("010211120A0B");
        for i in 0..expected.len() {
            assert_eq!(expected[i], result[i]);
        }
    }
}

/* Canframe::from example
And parsing of data section
use std::num::ParseIntError;

#[derive(Debug)]
struct CanFrame {
    timestamp: f64,
    interface: String,
    can_id: u32,
    data: Vec<u8>,
}

impl CanFrame {
    fn from_candump_line(line: &str) -> Result<CanFrame, String> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Ensure that the line has the expected number of parts
        if parts.len() < 4 {
            return Err("Invalid line format".to_string());
        }

        // Parse the timestamp (f64)
        let timestamp: f64 = parts[0]
            .parse()
            .map_err(|_| "Invalid timestamp format")?;

        // Parse the interface (can0, can1, etc.)
        let interface = parts[1].to_string();

        // Parse the CAN ID (hexadecimal)
        let can_id = u32::from_str_radix(parts[2], 16).map_err(|_| "Invalid CAN ID format")?;

        // Parse the data length (in square brackets, e.g., [8])
        let data_len_str = parts[3];
        if !data_len_str.starts_with('[') || !data_len_str.ends_with(']') {
            return Err("Invalid data length format".to_string());
        }
        let data_len: usize = data_len_str[1..data_len_str.len() - 1]
            .parse()
            .map_err(|_| "Invalid data length value")?;

        // Ensure that the number of data bytes matches the data length
        if parts.len() != 4 + 1 { // 4 parts plus the data itself
            return Err("Mismatch between data length and actual data bytes".to_string());
        }

        // Parse the data: this is a continuous hexadecimal string
        let data_str = parts[4];
        if data_str.len() != data_len * 2 {
            return Err("Mismatch between data length and actual data byte count".to_string());
        }

        let data = data_str
            .as_bytes()
            .chunks(2)
            .map(|chunk| {
                u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16)
                    .map_err(|e| format!("Invalid byte in data: {}", e))
            })
            .collect::<Result<Vec<u8>, String>>()?;

        Ok(CanFrame {
            timestamp,
            interface,
            can_id,
            data,
        })
    }
}

fn main() {
    // Example usage
    let log_line = "1582359202.874678  can0  123   [8]  0102030405060708";
    match CanFrame::from_candump_line(log_line) {
        Ok(frame) => println!("{:?}", frame),
        Err(e) => eprintln!("Error parsing line: {}", e),
    }
}

 */
