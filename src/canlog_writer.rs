/*!
 * Interfaces to write CAN frames to output log files
 */

use std::fmt::Write as FmtWrite; use std::fs::File;
//for write! on Strings. Just need trait in scope
use std::io::{self, BufWriter};
use std::io::Write;
use std::path::Path;
use crate::canlog_reader::CanFrame;

/// Convert a CanFrame to an ascii candump line
/// Example: (1436509053.850870) vcan0 1A0#9C20407F96EA167B
pub fn frame_to_candump_line(frame: &CanFrame) -> String {
    //Formatting:
    //Timestamp: 6 decimal digits (to microsecond)
    //Channel: full string
    //Frame ID: in hex
    //Data: in hex with leading 0 if needed
    let mut s = if frame.is_fd{ 
        //CAN FD format has ##<flags>
        /* Flags are 
        Flags = 0 (No flags, standard FD frame)
        Flags = 1 (CANFD_BRS - Bit Rate Switch)
        Flags = 2 (CANFD_ESI - Error State Indicator)
        Flags = 3 (CANFD_ESI | CANFD_BRS */
        // We ignore these flags and hardcode to 0 as they are hardware level details
        // here we are writing to file, not hardware device.
        format!("({:.6}) {} {:03X}##0", frame.timestamp, frame.channel, frame.id)
    } else {
        format!("({:.6}) {} {:03X}#", frame.timestamp, frame.channel, frame.id)
    };
    for i in 0..frame.len as usize {
        write!(s, "{:02X}", frame.data[i]).unwrap();
    }
    return s;
}

/// Trait for anything that can accept CAN frames for output.
/// Designed to be implemented for file writers now and
/// physical CAN channels (SocketCAN) in the future.
pub trait CanWriter {
    /// Write a single frame to the output.
    fn write(&mut self, frame: &CanFrame) -> io::Result<()>;

    // Flush any buffered output.
    fn flush(&mut self) -> io::Result<()>;
}

/// Write CanFrames to a log file in candump (linux can-utils) format
pub struct CandumpWriter<W: io::Write> {
    writer: BufWriter<W>,
}

impl<W: io::Write> CandumpWriter<W> {
    pub fn write(&mut self, frame: &CanFrame) -> io::Result<()> {
        /*let t = self.writer.write_all(frame_to_candump_line(frame).as_bytes()).unwrap();
        self.writer.flush()*/
        let mut line = frame_to_candump_line(frame);
        line.push('\n');
        self.writer.write_all(line.as_bytes())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
    // Create
    pub fn from_writer(writer: W) -> Self {
        Self {
            writer: BufWriter::new(writer)
        }
    }
}

impl CandumpWriter<File> {
    /// Create a new writer to a file.
    /// Creates a new file if one does not exist, 
    /// erases existing file contents if it does exist
    pub fn from_path<P: AsRef<Path>> (path: P) -> io::Result<Self>{
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file)
        })
    }
}

/// Create a writer that auto-detects format from file extension.
/// .log -> CandumpWriter, .asc -> AsciiWriter
/*pub fn writer_from_path(path: &Path) -> io::Result<Box<dyn CanWriter>> {
    let extension = path.extension()
    if path.extension().ok_or_else == ".log" {
        CandumpWriter::from_path(path)
    }
}*/

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};

    use crate::canlog_reader;
    use tempfile::NamedTempFile;

    use super::*;

    // Inputs: CanFrame Output: Known canline string
    #[test]
    fn test_frame_to_candump_line() {
        let expected_line = "(1436509053.850870) vcan0 1A0#9C20407F96EA167B";
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

        assert_eq!(frame_to_candump_line(&input_frame), expected_line);
    }

    #[test]
    fn test_frame_to_candump_fd_line() {
        //Roundtrip a candump CAN FD line: Candump Line -> CanFrame -> Candump line
        let expected_line = "(1769227442.503764) vcan1 1F334455##01122334455667788";
        let input_frame = canlog_reader::parse_candump_line(expected_line).unwrap();
        assert_eq!(frame_to_candump_line(&input_frame), expected_line);
    }

    #[test]
    fn test_padding_id() {
        //TODO: Test extended ID padding
        let expected_line = "(1579876762.059466) slcan0 002#BE0000079B";
        let input_frame = canlog_reader::parse_candump_line(expected_line).unwrap();
        assert_eq!(frame_to_candump_line(&input_frame), expected_line);
    }

    //TODO: Ascii write support
    //test_frame_to_ascii_line

    //test_frame_to_ascii_fd_line() {}

    //File Writing
    //Test writing to file (use std::write trait with std::io::cursor to do in memory)
    //test_candump_write
    #[test]
    fn test_candump_write() {
        //Create a tempfile that is removed after this test
        let file = NamedTempFile::new().unwrap();
        let filepath = file.path(); //"test-file-2.txt";

        let mut writer = CandumpWriter::from_path(filepath).unwrap();
        let expected_line = "(1769227442.503764) vcan1 1F334455##01122334455667788";
        let input_frame = canlog_reader::parse_candump_line(expected_line).unwrap();

        writer.write(&input_frame).unwrap();
        writer.flush().unwrap();

        let read_back_line = fs::read_to_string(filepath).unwrap();
        //Writing to file adds newlines, so we manually add to expected result
        assert_eq!(expected_line.to_string() + "\n",read_back_line);
    }
    //test_vector_ascii_write


}
