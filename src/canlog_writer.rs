/*!
 * Interfaces to write CAN frames to output log files
 */

use std::fmt::Write;
use crate::canlog_reader::CanFrame;

/// Convert a CanFrame to an ascii candump line
/// Example: (1436509053.850870) vcan0 1A0#9C20407F96EA167B
pub fn frame_to_candump_line(frame: CanFrame) -> String {
    let mut s = if frame.is_fd{ 
        //CAN FD format has ##<flags>
        /* Flags are 
        Flags = 0 (No flags, standard FD frame)
        Flags = 1 (CANFD_BRS - Bit Rate Switch)
        Flags = 2 (CANFD_ESI - Error State Indicator)
        Flags = 3 (CANFD_ESI | CANFD_BRS */
        // We ignore these flags and hardcode to 0 as they are hardware level details
        // here we are writing to file, not hardware device.
        format!("({}) {} {:X}##0", frame.timestamp, frame.channel, frame.id)
    } else {
        format!("({}) {} {:X}#", frame.timestamp, frame.channel, frame.id)
    };
    for i in 0..frame.len as usize {
        write!(s, "{:02X}", frame.data[i]).unwrap();
    }
    return s;
}

pub struct CanLogWriter {

}

#[cfg(test)]
mod tests {
    use crate::canlog_reader;

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
            is_fd: false,
            len: 8,
            data: CanFrame::default_data(),
        };
        for (i,byte) in [0x9C as u8,0x20,0x40,0x7F,0x96,0xEA,0x16,0x7B].iter().enumerate(){    
            input_frame.data[i] = *byte;
        }

        assert_eq!(frame_to_candump_line(input_frame), expected_line);
    }

    #[test]
    fn test_frame_to_candump_fd_line() {
        //Roundtrip a candump CAN FD line: Candump Line -> CanFrame -> Candump line
        let expected_line = "(1769227442.503764) vcan1 1F334455##01122334455667788";
        let input_frame = canlog_reader::parse_candump_line(expected_line).unwrap();
        assert_eq!(frame_to_candump_line(input_frame), expected_line);
    }

    //test_frame_to_ascii_line

    //test_frame_to_ascii_fd_line() {}

    //Test writing to file (use std::write trait with std::io::cursor to do in memory)
    //test_candump_write

    //test_vector_ascii_write

}
