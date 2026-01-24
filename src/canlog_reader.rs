use std::borrow::Borrow;
use std::fmt::Error;
use std::fmt::Write;
use std::fs::File;
use std::io::Cursor;
use std::io::{self, BufRead, BufReader};
use std::num::ParseIntError;
use std::time::Instant;

const DEFAULT_FRAME_PAYLOAD_LEN: usize = 64;
#[derive(Debug,PartialEq, PartialOrd)]
pub struct CanFrame {
    // Timestamp: Time the data was received (seconds)
    pub timestamp: f64,
    // Name of the CAN channel the data occurred on.
    pub channel: String,
    // CAN ID: 11-bit standard or 29-bit extended ID
    pub id: u32,
    // Was the data received? True for receive, false for transmitted.
    // Default is True(receive) if not specified in the log
    pub is_rx: bool,
    // Data Length Code (DLC), 0 to 8 for CAN, 0 to 64 for CAN FD
    pub len: u8,
    // Payload data, can store up to 64 bytes for CAN FD, 8 bytes for standard CAN
    pub data: [u8; DEFAULT_FRAME_PAYLOAD_LEN],
    //pub data: Vec<u8>, This is ~3-5ms slower than 64 byte over 200k lines
}


impl Default for CanFrame {
    fn default() -> Self {
        CanFrame {
            // Use the array initialization syntax [x; N]
            timestamp: 0.0,
            channel: String::new(),
            id: 0,
            is_rx: false,
            len: 0,
            data:[0; DEFAULT_FRAME_PAYLOAD_LEN], 
        }
    }
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

/// Turn hex data from candump log into byte values
pub fn candump_hex_to_bytes(hex_str: &str) -> Result<[u8; DEFAULT_FRAME_PAYLOAD_LEN],ParseIntError> {
//pub fn candump_hex_to_bytes(hex_str: &str) -> Result<Vec<u8>,ParseIntError> {
    let mut data_bytes = [0; DEFAULT_FRAME_PAYLOAD_LEN];
    //let mut data_bytes = Vec::new();

    let mut index = 0;
    let mut i = 0;
    while i < hex_str.len() {
        //data_bytes.push(u8::from_str_radix(&hex_str[i..i + 2], 16)?);
        data_bytes[index] = u8::from_str_radix(&hex_str[i..i + 2], 16)?;
            //.expect(&format!("failed to parse data bytes {}", hex_str));
        index += 1;
        i += 2;
    }
    return Ok(data_bytes);
}

/// Parse a line in candump format
/// (1436509053.850870) vcan0 1A0#9C20407F96EA167B
/// ```
/// rocketcan::canlog_reader::parse_candump_line(" (1436509053.850870) vcan0 1A0#9C20407F96EA167B");
/// ```
pub fn parse_candump_line(line: &str) -> anyhow::Result<CanFrame> { //TODO: Change anyhow to custom error type
    //Error in case parsing fails

    let mut line_splits = line.split_whitespace();
    //Get timestamp
    let timestamp = line_splits.next().ok_or_else(|| anyhow::anyhow!("Error parsing timestamp of {line}"))?;
    let timestamp = &timestamp[1..timestamp.len() - 1];
    let timestamp = timestamp.parse::<f64>()?;
    // CAN interface name
    let interface_name = line_splits.next().ok_or_else(|| anyhow::anyhow!("Error parsing interface of {line}"))?;
    //ID
    let id_and_data_substr = line_splits.next().ok_or_else(|| anyhow::anyhow!("Error no id#data on {line}"))?;
    //"##" means it was CAN FD
    let mut id_and_data: Vec<_> = id_and_data_substr.split("##").collect();
    let is_fd = id_and_data.len() > 1;
    // If there was no "##", it is standard CAN format ("#")
    let mut start_idx = 1; //CAN FD format has 1 character of bitflags that we skip.
    if !is_fd {
        start_idx = 0; //Standard CAN does not have bitflags character
        id_and_data = id_and_data_substr.split('#').collect();
    }
    /*let id_and_data: Vec<_> = line_splits.next().ok_or_else(|| anyhow::anyhow!("Error no id#data on {line}"))?
        .split('#').collect();*/
    let id = u32::from_str_radix(id_and_data[0], 16)?;
    let candump_data_payload = &id_and_data[1][start_idx..];
    let data = candump_hex_to_bytes(candump_data_payload)?;
    let data_len = (candump_data_payload.len() / 2) as u8;
    return Ok(CanFrame {
        timestamp: timestamp,
        channel: interface_name.to_owned(),
        id: id,
        is_rx: true, //Candump doesn't specify, default is true.
        len: data_len,
        data: data,
    });
}

/// Base format for Vector ascii parsing. Hex or Decimal (base 10).
pub enum AsciiBase {
    Hex,
    Dec,
}
/// Parse a line in ascii format from Vector tool
/// 
/// <Time> <Channel> <ID> <Dir> d <DLC> <D0> <D1>...<D8> <MessageFlags>
/// 1.000000 1  100             Tx   d 8   1   2   3   4   5   6   7   8  Length = 0 BitCount = 64 ID = 100
/// ```
/// let test_string = "1.5 1  150             Tx   d 8   1   2   3   4   5   6   7   8  Length = 0 BitCount = 64 ID = 150";
/// assert!(rocketcan::canlog_reader::parse_ascii_line(test_string, AsciiBase::Dec).is_ok());
/// ```
/// 
/// CAN Remote Frame Event
/// <Time> <Channel> <ID> <Dir> r
/// 1.000000 1  100             Tx   r
pub fn parse_ascii_line(line: &str, base: AsciiBase) -> anyhow::Result<CanFrame> {
    //let mut line_splits = line.split_whitespace();

    //let timestamp = line_splits.next().ok_or_else(|| anyhow::anyhow!("Error parsing timestamp of {line}"))?;
    let mut frame: CanFrame = Default::default();
    let data_start = 6;
    let mut data_end = 6;
    let radix = match base {
        AsciiBase::Hex => 16,
        AsciiBase::Dec => 10,
    };

    //frame.timestamp = line_splits.next()?.parse::<f64>()?;
    for (i,item) in line.split_whitespace().enumerate() {
        match i {
            0 => frame.timestamp = item.parse::<f64>()?,
            1 => frame.channel = item.to_owned(),
            2 => frame.id = u32::from_str_radix(item, radix)?,
            3 => frame.is_rx = item == "Rx",
            4 => { //Normal frame, or remote frame?
                //If it is a remote frame, end now.
                // No data, len is 0
                if item == "r" {
                    frame.len = 0;
                    return Ok(frame);
                }                
            }
            5 => { //Data length
                frame.len = u8::from_str_radix(item, 10)?;
                data_end = data_start + frame.len as usize;
            },
            //Max datalen is 64
            6..70 => { //Filling data fields
                if i == data_end{
                    return Ok(frame);
                } else if i > data_end{
                    return Err(anyhow::anyhow!("Parse ascii error"));
                }

                let byte = u8::from_str_radix(item,radix)?;
                frame.data[i-data_start] = byte;
            },
            _ => return Err(anyhow::anyhow!("Parse ascii error")),
        }
    }
    Ok(frame)
}

/// Convert a CanFrame to an ascii candump line
pub fn frame_to_candump_line(frame: CanFrame) -> String {
    let mut s = format!("({}) vcan0 {:X}#", frame.timestamp, frame.id);
    for i in 0..frame.len as usize {
        write!(s, "{:02X}", frame.data[i]).unwrap();
    }
    return s;
}

pub struct CanLogParser/*<R>*/{
    //reader: R,
    reader: Box<dyn BufRead>,
    buf: String, // local buf to re-use so we don't keep allocating
}

impl CanLogParser {

    /// Create CanLogParser from a file path
    pub fn from_file(path: &std::path::Path) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok( CanLogParser { 
            reader: Box::new(BufReader::new(file)), 
            buf: String::new(),
        })
    }

    /// Create CanLogParser from raw bytes
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        CanLogParser { 
            reader: Box::new(Cursor::new(bytes)), 
            buf: String::new(),
        }
    }

    /// Create CanLogParser from any type that implements the BufRead trait
    pub fn from_reader<R: BufRead + 'static>(reader: R) -> Self {
        Self {
            reader: Box::new(reader),
            buf: String::new(),
        }
    }

}

impl Iterator for CanLogParser {
    type Item = CanFrame;

    fn next(&mut self) -> Option<Self::Item>{
        self.buf.clear();
        match self.reader.read_line(&mut self.buf) {
            Ok(0) => None,
            Ok(_) => {
                //Some(String::from(self.buf.trim_end()))
                //Some(parse_candump_line(&self.buf))
                //TODO: Should this cause an error? 
                //Consider returning Option<Result<CanFrame>> to indicate parse failure on line to user
                parse_candump_line(&self.buf).ok() //throw away parsing failures here...
            }
            Err(_) => None,
        }
    }
}
pub struct CanLogReader<T>
where
    T: Iterator,
{
    iterable: T,
}

impl<T> Iterator for CanLogReader<T>
where
    T: Iterator<Item = std::io::Result<String>>,
    //T::Item: std::borrow::Borrow<str>,
{
    type Item = CanFrame;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(line) = self.iterable.next() {
            //println!("{}", line.unwrap());
            //return Some(parse_candump_line(&line.unwrap()));
            return parse_candump_line(&line.unwrap()).ok();
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
    use std::io::Seek;

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
        let mut can_reader_collection = Vec::new();
        for can_frame in cr {
            println!("{:?}", can_frame);
            can_reader_collection.push(can_frame);
        }

        let can_parser = CanLogParser::from_file(std::path::Path::new(filename)).unwrap();
        let mut can_parser_collection = Vec::new();
        for can_frame in can_parser {
            println!("{:?}", can_frame);
            can_parser_collection.push(can_frame);
        } 

        assert_eq!(can_parser_collection.len(), can_reader_collection.len());
        for i in 0..can_parser_collection.len() {
            assert_eq!(can_parser_collection[i], can_reader_collection[i]);
        }
    }

    #[test]
    fn benchmark_reading() {
        //let filename = "candump.log";
        let filename = "can_samples/aphryx-canx-nissan-leaf/demo_meet_200k_revised.log";
        let reader = CanLogReader::from_file(filename);
        let parser = CanLogParser::from_file(std::path::Path::new(filename)).unwrap();
        let parser2 = CanLogParser::from_file(std::path::Path::new(filename)).unwrap();

        let parser_t1 = Instant::now();
        let mut v0 = Vec::new();
        for frame in parser {
            //println!("{:?}",frame);
            v0.push(frame);
        }

        let total_time = Instant::now();
        let reader_t1 = Instant::now();
        let mut v1 = Vec::new();
        for frame in reader {
            //println!("{:?}",frame);
            v1.push(frame);
        }
        let reader_time = reader_t1.elapsed().as_micros();

        let parser_t1 = Instant::now();
        let mut v2 = Vec::new();
        for frame in parser2 {
            //println!("{:?}",frame);
            v2.push(frame)
        }
        let parser_time = parser_t1.elapsed().as_micros();

        /*let reader_t1 = Instant::now();
        let mut v1 = Vec::new();
        for frame in reader {
            //println!("{:?}",frame);
            v1.push(frame);
        }
        let reader_time = reader_t1.elapsed().as_micros();*/

        let total_time = total_time.elapsed().as_micros();
        println!("Reader: {reader_time} us, Parser {parser_time} us , Total {total_time} us");
        println!("v1 len {}, v2 len {}, v0 len {}", v1.len(),v2.len(),v0.len());
    }

    // candump / can-utils testing
    #[test]
    fn test_candump_hex_data() {
        let expected = vec![1u8, 2u8, 17u8, 18u8, 10u8, 11u8];
        let result = candump_hex_to_bytes("010211120A0B").unwrap();
        for i in 0..expected.len() {
            assert_eq!(expected[i], result[i]);
        }
    }

    /// Write to the bytes starting with start value and incrementing by value_step each byte.
    fn fill_bytes(bytes: &mut [u8], start_value: u8, value_step: u8) {
        let mut value = start_value;
        for byte in bytes.iter_mut() {
            *byte = value;
            value += value_step;
        }
    }

    /// Write to the bytes starting with start value and incrementing by value_step each byte.
    /// Starts over every pattern_len steps
    fn fill_bytes_repeating(bytes: &mut [u8], pattern_len: usize, start_value: u8, value_step: u8) {
        let mut value = start_value;
        for (i,byte) in bytes.iter_mut().enumerate() {
            if i % pattern_len == 0 { // Reset every pattern_len steps
                value = start_value;
            }
            *byte = value;
            value += value_step;
        }
    }

    #[test]
    // CAN 2.0 format test
    fn test_candump_can_2_0() {
        let candump_standard_id = "(1769227752.525818) vcan1 123#1122334455667788";
        let mut expected_frame = CanFrame {
            timestamp: 1769227752.525818,
            channel: String::from("vcan1"),
            id: 291,
            is_rx: true,
            len: 8,
            data: [0;DEFAULT_FRAME_PAYLOAD_LEN],
        };
        fill_bytes(&mut expected_frame.data[0..8],17,17);
        assert_eq!(expected_frame, parse_candump_line(candump_standard_id).unwrap());

        let extended_id_line = "(1769227752.525818) vcan1 1F334455#1122334455667788";
        expected_frame.id = 523453525;
        assert_eq!(expected_frame, parse_candump_line(extended_id_line).unwrap());
    }

    #[test]
    // CAN FD format test
    fn test_candump_can_fd() {
        let fd_line = "(1769227442.503764) vcan1 123##400";
        let mut expected_frame = CanFrame {
            timestamp: 1769227442.503764,
            channel: String::from("vcan1"),
            id: 291,
            is_rx: true,
            len: 1,
            data: [0;DEFAULT_FRAME_PAYLOAD_LEN],
        };
        assert_eq!(expected_frame, parse_candump_line(fd_line).unwrap());

        let fd_ext_id_line = "(1769227442.503764) vcan1 1F334455##41122334455667788";
        expected_frame.len = 8;
        expected_frame.id = 523453525;
        fill_bytes(&mut expected_frame.data[0..8],17,17);
        assert_eq!(expected_frame, parse_candump_line(fd_ext_id_line).unwrap());
    }

    // CAN FD varying data lengths
    #[test]
    fn test_candump_can_fd_lengths() {
        let fd_32bytes_line = "(1769227729.672570) vcan1 1F334455##51122334455667788112233445566778811223344556677881122334455667788";
        let mut expected_frame = CanFrame {
            timestamp: 1769227729.672570,
            channel: String::from("vcan1"),
            id: 523453525,
            is_rx: true,
            len: 32,
            data: [0;DEFAULT_FRAME_PAYLOAD_LEN],
        };
        fill_bytes_repeating(&mut expected_frame.data[0..32],8,17,17);
        assert_eq!(expected_frame, parse_candump_line(fd_32bytes_line).unwrap());

        let fd_64bytes_line = "(1769227729.672570) vcan1 123##F11223344556677881122334455667788112233445566778811223344556677881122334455667788112233445566778811223344556677881122334455667788";
        expected_frame.id = 291;
        expected_frame.len = 64;
        fill_bytes_repeating(&mut expected_frame.data[0..64],8,17,17);
        assert_eq!(expected_frame, parse_candump_line(fd_64bytes_line).unwrap());
    }

    // Vector ascii format tests
    #[test]
    fn test_parse_ascii_line_error() {
        //It returns error on candump line
        let candump_line = "(1436509053.850870) vcan0 1A0#9C20407F96EA167B";
        assert!(parse_ascii_line(candump_line, AsciiBase::Hex).is_err());

        //Base decimal test
        /*let ascii_8 = "1.000000 1  100             Tx   d 8   1   2   3   4   5   6   7   8  Length = 0 BitCount = 64 ID = 100";
        let mut expected_frame = CanFrame {
            timestamp: 1.0,
            channel: String::from("1"),
            id: 100,
            is_rx: false,
            len: 8,
            data: [0;DEFAULT_FRAME_PAYLOAD_LEN],
        };
        for i in 1..=8 {
            expected_frame.data[i-1] = i as u8;
        }
        assert_eq!(expected_frame, parse_ascii_line(ascii_8).unwrap());*/
    }

    #[test]
    fn test_ascii_remote_frame() {
        //Remote frame
        let remote_frame = "1.500000 1  150             Tx   r";
        let expected_frame = CanFrame {
            timestamp: 1.5,
            channel: String::from("1"),
            id: 336,
            is_rx: false,
            len: 0,
            data: [0;DEFAULT_FRAME_PAYLOAD_LEN],
        };
        assert_eq!(expected_frame, parse_ascii_line(remote_frame,AsciiBase::Hex).unwrap());
    }

    #[test]
    // Check parsing when using hex base and dec base (16 vs. 10)
    fn test_ascii_base_dec_vs_hex() {
        let ascii_line = "0.400291 1  150       Rx   d 8 11 22 33 44 55 66 77 88";
        let mut expected_frame = CanFrame {
            timestamp: 0.400291,
            channel: String::from("1"),
            id: 150,
            is_rx: true,
            len: 8,
            data: [0;DEFAULT_FRAME_PAYLOAD_LEN],
        };
        fill_bytes(&mut expected_frame.data[0..8], 11, 11);
        let result = parse_ascii_line(ascii_line,AsciiBase::Dec);
        assert_eq!(expected_frame, result.unwrap());

        fill_bytes(&mut expected_frame.data[0..8], 17, 17);
        expected_frame.id = 336;
        assert_eq!(expected_frame, parse_ascii_line(ascii_line,AsciiBase::Hex).unwrap());
    }

    #[test]
    fn test_parse_ascii_hex_base() {
        //4 bytes
        let hex_id_line = "0.217398 2  30B             Rx   d 4 00 00 00 00  Length = 236000 BitCount = 122 ID = 779";
        let expected_frame = CanFrame {
            timestamp: 0.217398,
            channel: String::from("1"),
            id: u32::from_str_radix("30B", 16).unwrap(),
            is_rx: true,
            len: 4,
            data: [0;DEFAULT_FRAME_PAYLOAD_LEN],
        };
        assert_eq!(expected_frame, parse_ascii_line(hex_id_line, AsciiBase::Hex).unwrap());
    }

    #[test]
    fn test_ascii_extended_id() {
        let extended_id_line = "0.400291 1  1F334455x       Rx   d 8 01 02 03 04 05 06 07 08";
        let mut expected_frame = CanFrame {
            timestamp: 0.400291,
            channel: String::from("1"),
            id: 523453525,
            is_rx: true,
            len: 8,
            data: [0;DEFAULT_FRAME_PAYLOAD_LEN],
        };
        assert_eq!(expected_frame, parse_ascii_line(extended_id_line, AsciiBase::Hex).unwrap());
    }
}
