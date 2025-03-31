use std::borrow::Borrow;
use std::fmt::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

#[derive(Debug)]
pub struct CanFrame {
    // CAN ID: 11-bit standard or 29-bit extended ID
    pub id: u32,
    // Data Length Code (DLC), 0 to 8 for CAN, 0 to 64 for CAN FD
    pub len: u8,
    // Payload data, can store up to 64 bytes for CAN FD, 8 bytes for standard CAN
    pub data: [u8; 64],
    // A flag to differentiate between standard CAN and CAN FD
    pub is_can_fd: bool,
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

pub struct CanLogReader<T> {
    iterable: T,
}
//Would have to implement trait for io::Lines
pub trait CanLogRead {
    fn to_canlog_reader(self) -> CanLogReader<Self>
    where
        Self: Sized,
    {
        CanLogReader { iterable: self }
    }
}

impl<T> CanLogRead for io::Lines<T> /*where T: IntoIterator<Item: std::borrow::Borrow<str>>*/ {}
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
    T: Iterator,
    T::Item: std::borrow::Borrow<str>,
{
    type Item = CanFrame;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(line) = self.iterable.next() {
            println!("{}", line.borrow());
        }
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    /*fn test_from_string() {
        let one_line_str = String::from("(1436509052.249713) vcan0 044#2A366C2BBA");
        let mut reader = CanLogReader::<Vec<&str>>::from_string(&one_line_str);
        println!("{:?}", reader.next());
    }*/
    #[test]
    fn test_file() {
        println!("HELLO WORLD----");
    }
    fn test_from_file() {
        let filename = "candump.log";
        let Ok(f) = File::open(filename) else {
            panic!("Unable to open file named {filename}");
        };
        let buf_reader = BufReader::new(f);
        let t = buf_reader.lines();
        //let next = t.next();
        let cr = t.to_canlog_reader();
        cr.next();
    }
}
