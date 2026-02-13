/**
 * Interfaces to write CAN frames to output log files
 */

use std::fmt::Write;
use crate::canlog_reader::CanFrame;

/// Convert a CanFrame to an ascii candump line
/// Example: (1436509053.850870) vcan0 1A0#9C20407F96EA167B
pub fn frame_to_candump_line(frame: CanFrame) -> String {
    let mut s = format!("({}) vcan0 {:X}#", frame.timestamp, frame.id);
    for i in 0..frame.len as usize {
        write!(s, "{:02X}", frame.data[i]).unwrap();
    }
    return s;
}

pub struct CanLogWriter {

}

#[cfg(test)]
mod tests {
    use super::*;

    // Inputs: CanFrame Output: Known canline string
    #[test]
    fn test_frame_to_candump_line() {
        let expected_line = "(1436509053.85087) vcan0 1A0#9C20407F96EA167B";
        let mut input_frame = CanFrame {
            timestamp: 1436509053.850870,
            id: 0x1A0,
            channel: "vcan0".to_string(),
            is_rx: true, //candump doesn't record rx/tx
            len: 8,
            data: CanFrame::default_data(),
        };
        for (i,byte) in [0x9C as u8,0x20,0x40,0x7F,0x96,0xEA,0x16,0x7B].iter().enumerate(){    
            input_frame.data[i] = *byte;
        }

        assert_eq!(frame_to_candump_line(input_frame), expected_line);
    }

    //test_frame_to_candump_fd_line() {}

    //test_frame_to_ascii_line

    //test_frame_to_ascii_fd_line() {}

    //Test writing to file (use std::write trait with std::io::cursor to do in memory)
    //test_candump_write

    //test_vector_ascii_write

}
