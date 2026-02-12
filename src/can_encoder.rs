use anyhow::{anyhow, Result};

use crate::can_decoder;
use crate::canlog_reader::CanFrame;
use crate::signal_layout::SignalLayout;

/// Convert a physical (engineering) value to the raw unsigned integer
/// that gets packed into the CAN frame data.
///
/// This is the inverse of `compute_signal_value`:
///   physical = raw * factor + offset
///   raw = (physical - offset) / factor
///
/// For signed signals, the result is truncated to `signal_size` bits
/// (two's complement representation stored as u64).
pub fn compute_raw_value(physical: f64, spec: &can_dbc::Signal) -> u64 {
    let raw_f64 = (physical - spec.offset()) / spec.factor();

    match spec.value_type() {
        can_dbc::ValueType::Signed => {
            let raw_i64 = raw_f64.round() as i64;
            // Mask to signal_size bits to get the unsigned two's complement representation
            if spec.signal_size >= 64 {
                raw_i64 as u64
            } else {
                (raw_i64 as u64) & ((1u64 << spec.signal_size) - 1)
            }
        }
        can_dbc::ValueType::Unsigned => {
            let raw_u64 = raw_f64.round() as u64;
            // Clamp to representable range
            if spec.signal_size >= 64 {
                raw_u64
            } else {
                raw_u64 & ((1u64 << spec.signal_size) - 1)
            }
        }
    }
}

/// Encode a full message from signal name/value pairs into a `CanFrame`.
///
/// Looks up each signal by name in `message_spec`, computes the raw value,
/// and packs it into the frame data using `SignalLayout`. Unspecified signals
/// are left as zero.
///
/// Returns an error if any signal name is not found in the message spec.
pub fn encode_message(
    message_spec: &can_dbc::Message,
    signals: &[(&str, f64)],
    message_id: u32,
) -> Result<CanFrame> {
    let mut frame = CanFrame::default();
    frame.id = message_id;
    frame.len = *message_spec.message_size() as u8;

    for (signal_name, physical_value) in signals {
        let spec = can_decoder::get_signal_spec(message_spec, signal_name)
            .ok_or_else(|| anyhow!("unknown signal: {}", signal_name))?;
        let layout = SignalLayout::from_spec(spec);
        let raw = compute_raw_value(*physical_value, spec);
        layout.pack(&mut frame.data, raw);
    }

    Ok(frame)
}

/// Builder for constructing encoded CAN frames signal-by-signal.
///
/// Uses the consuming-self pattern so that each `.set()` call moves
/// the builder, preventing accidental reuse of a half-built frame.
pub struct CanFrameBuilder<'a> {
    message_spec: &'a can_dbc::Message,
    frame: CanFrame,
}

impl<'a> CanFrameBuilder<'a> {
    pub fn new(message_spec: &'a can_dbc::Message, message_id: u32) -> Self {
        let mut frame = CanFrame::default();
        frame.id = message_id;
        frame.len = *message_spec.message_size() as u8;
        Self { message_spec, frame }
    }

    /// Set a signal by name. Returns Err if the signal name is not found.
    pub fn set(mut self, signal_name: &str, physical_value: f64) -> Result<Self> {
        let spec = can_decoder::get_signal_spec(self.message_spec, signal_name)
            .ok_or_else(|| anyhow!("unknown signal: {}", signal_name))?;
        let layout = SignalLayout::from_spec(spec);
        let raw = compute_raw_value(physical_value, spec);
        layout.pack(&mut self.frame.data, raw);
        Ok(self)
    }

    pub fn timestamp(mut self, ts: f64) -> Self {
        self.frame.timestamp = ts;
        self
    }

    pub fn channel(mut self, ch: String) -> Self {
        self.frame.channel = ch;
        self
    }

    /// Consume the builder and produce the finished frame.
    pub fn build(self) -> CanFrame {
        self.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::can_decoder;
    use crate::canlog_reader;
    use crate::signal_layout::SignalLayout;

    // ---------------------------------------------------------------
    // compute_raw_value tests
    // ---------------------------------------------------------------

    #[test]
    fn test_compute_raw_unsigned() {
        // AverageRadius: factor=0.1, offset=0, unsigned 6-bit
        // physical=1.8 → raw = (1.8 - 0) / 0.1 = 18
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();
        let signal = can_decoder::get_signal_spec(&msg, "AverageRadius").unwrap();

        let raw = compute_raw_value(1.8, signal);
        assert_eq!(raw, 18);
    }

    #[test]
    fn test_compute_raw_signed_negative() {
        // Temperature: factor=0.01, offset=250, signed 12-bit
        // physical=244.14 → raw = (244.14 - 250) / 0.01 = -586
        // -586 as 12-bit two's complement = 4096 - 586 = 0xDB6
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();
        let signal = can_decoder::get_signal_spec(&msg, "Temperature").unwrap();

        let raw = compute_raw_value(244.14, signal);
        assert_eq!(raw, 0xDB6);
    }

    #[test]
    fn test_compute_raw_signed_positive() {
        // Enable: factor=1, offset=0, unsigned 1-bit
        // physical=1.0 → raw = 1
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();
        let signal = can_decoder::get_signal_spec(&msg, "Enable").unwrap();

        let raw = compute_raw_value(1.0, signal);
        assert_eq!(raw, 1);
    }

    // ---------------------------------------------------------------
    // encode_message tests
    // ---------------------------------------------------------------

    #[test]
    fn test_encode_message_motohawk_all_signals() {
        // Encode all three motohawk signals and verify:
        // 1. Frame metadata is correct
        // 2. Known byte values for fully-covered bytes match
        // 3. Decoding each signal back yields the original physical value
        //
        // Note: The golden frame A5B6D9... has residual bits in byte 2
        // that don't belong to any signal. Encoding from scratch leaves
        // those bits as zero, so we verify signal-level correctness
        // rather than raw byte equality for partially-covered bytes.
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let signals: &[(&str, f64)] = &[
            ("Temperature", 244.14),
            ("AverageRadius", 1.8),
            ("Enable", 1.0),
        ];

        let frame = encode_message(msg, signals, 0x1F0).unwrap();

        assert_eq!(frame.id, 0x1F0);
        assert_eq!(frame.len, 8);
        // Byte 0 is fully covered by signals (Enable bit7, AverageRadius bits6..1, Temperature bit0)
        assert_eq!(frame.data[0], 0xA5, "byte 0");
        // Byte 1 is fully covered by Temperature
        assert_eq!(frame.data[1], 0xB6, "byte 1");

        // Verify each signal decodes back to the original physical value
        for (signal_name, expected) in signals {
            let spec = can_decoder::get_signal_spec(&msg, signal_name).unwrap();
            let layout = SignalLayout::from_spec(spec);
            let decoded = layout.decode(&frame, spec);
            assert!(
                (decoded - expected).abs() < 1e-9,
                "signal '{}': expected {}, got {}",
                signal_name, expected, decoded
            );
        }
    }

    #[test]
    fn test_encode_message_partial() {
        // Encode only Enable and AverageRadius, leave Temperature at zero.
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let frame = encode_message(
            msg,
            &[("AverageRadius", 1.8), ("Enable", 1.0)],
            0x1F0,
        )
        .unwrap();

        // Decode Temperature should give the offset (250.0) since raw=0
        let temp_signal = can_decoder::get_signal_spec(&msg, "Temperature").unwrap();
        let layout = SignalLayout::from_spec(temp_signal);
        let temp_decoded = layout.decode(&frame, temp_signal);
        assert_eq!(temp_decoded, 250.0); // raw=0 → 0 * 0.01 + 250 = 250

        // AverageRadius should decode correctly
        let radius_signal = can_decoder::get_signal_spec(&msg, "AverageRadius").unwrap();
        let radius_layout = SignalLayout::from_spec(radius_signal);
        let radius_decoded = radius_layout.decode(&frame, radius_signal);
        assert!((radius_decoded - 1.8).abs() < 1e-10);

        // Enable should decode correctly
        let enable_signal = can_decoder::get_signal_spec(&msg, "Enable").unwrap();
        let enable_layout = SignalLayout::from_spec(enable_signal);
        let enable_decoded = enable_layout.decode(&frame, enable_signal);
        assert_eq!(enable_decoded, 1.0);
    }

    #[test]
    fn test_encode_message_unknown_signal_returns_error() {
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let result = encode_message(msg, &[("NonExistent", 42.0)], 0x1F0);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // Round-trip: encode → decode for motohawk
    // ---------------------------------------------------------------

    #[test]
    fn test_roundtrip_encode_decode_motohawk() {
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let signals: &[(&str, f64)] = &[
            ("Temperature", 244.14),
            ("AverageRadius", 1.8),
            ("Enable", 1.0),
        ];

        let frame = encode_message(msg, signals, 0x1F0).unwrap();

        // Decode each signal and verify it matches the original physical value
        for (signal_name, expected) in signals {
            let spec = can_decoder::get_signal_spec(&msg, signal_name).unwrap();
            let layout = SignalLayout::from_spec(spec);
            let decoded = layout.decode(&frame, spec);
            assert!(
                (decoded - expected).abs() < 1e-9,
                "roundtrip failed for '{}': encoded {}, decoded {}",
                signal_name, expected, decoded
            );
        }
    }

    // ---------------------------------------------------------------
    // Round-trip: encode → decode for signed.dbc
    // ---------------------------------------------------------------

    #[test]
    fn test_roundtrip_encode_decode_signed() {
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();

        // (message_name, message_id, signal_name, physical_value)
        let cases: &[(&str, u32, &str, f64)] = &[
            ("Message32", 0, "s32", 1000.0),
            ("Message32", 0, "s32", -1000.0),
            ("Message32big", 5, "s32big", 1000.0),
            ("Message32big", 5, "s32big", -1000.0),
            ("Message64", 2, "s64", 123456789.0),
            ("Message64", 2, "s64", -123456789.0),
            ("Message64big", 3, "s64big", 123456789.0),
            ("Message64big", 3, "s64big", -123456789.0),
        ];

        for (msg_name, msg_id, signal_name, physical) in cases {
            let msg = can_decoder::get_message_spec(&dbc, msg_name).unwrap();
            let frame = encode_message(msg, &[(signal_name, *physical)], *msg_id).unwrap();

            let spec = can_decoder::get_signal_spec(&msg, signal_name).unwrap();
            let layout = SignalLayout::from_spec(spec);
            let decoded = layout.decode(&frame, spec);
            assert_eq!(
                decoded, *physical,
                "roundtrip failed for {}.{}: encoded {}, decoded {}",
                msg_name, signal_name, physical, decoded
            );
        }
    }

    // ---------------------------------------------------------------
    // Round-trip: encode → decode_signal_by_bytes (cross-check)
    // ---------------------------------------------------------------

    #[test]
    fn test_encode_then_decode_signal_by_bytes_motohawk() {
        // Encode with can_encoder, decode with the existing decoder.
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let signals: &[(&str, f64)] = &[
            ("Temperature", 244.14),
            ("AverageRadius", 1.8),
            ("Enable", 1.0),
        ];

        let frame = encode_message(msg, signals, 0x1F0).unwrap();

        for (signal_name, expected) in signals {
            let spec = can_decoder::get_signal_spec(&msg, signal_name).unwrap();
            let decoded = can_decoder::decode_signal_by_bytes(&frame, spec);
            assert!(
                (decoded - expected).abs() < 1e-9,
                "encode→decode_signal_by_bytes failed for '{}': expected {}, got {}",
                signal_name, expected, decoded
            );
        }
    }

    // ---------------------------------------------------------------
    // CanFrameBuilder tests
    // ---------------------------------------------------------------

    #[test]
    fn test_builder_matches_encode_message() {
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let from_encode = encode_message(
            msg,
            &[
                ("Temperature", 244.14),
                ("AverageRadius", 1.8),
                ("Enable", 1.0),
            ],
            0x1F0,
        )
        .unwrap();

        let from_builder = CanFrameBuilder::new(msg, 0x1F0)
            .set("Temperature", 244.14)
            .unwrap()
            .set("AverageRadius", 1.8)
            .unwrap()
            .set("Enable", 1.0)
            .unwrap()
            .timestamp(0.0)
            .channel("vcan0".into())
            .build();

        assert_eq!(&from_encode.data[..8], &from_builder.data[..8]);
        assert_eq!(from_encode.id, from_builder.id);
        assert_eq!(from_encode.len, from_builder.len);
    }

    #[test]
    fn test_builder_unknown_signal_returns_error() {
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let result = CanFrameBuilder::new(msg, 0x1F0).set("Bogus", 1.0);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // Golden frame round-trip: decode existing frame → encode → compare bytes
    // ---------------------------------------------------------------

    #[test]
    fn test_encode_reproduces_golden_frame_motohawk() {
        // Decode the golden frame to get physical values, then re-encode
        // and verify that decoding the re-encoded frame yields the same values.
        //
        // We compare at the signal level rather than raw bytes because the
        // golden frame may have residual bits in positions not covered by
        // any defined signal.
        let golden_line = "(0.0) vcan0 1F0#A5B6D90000000000";
        let golden_frame = canlog_reader::parse_candump_line(golden_line).unwrap();
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        // Decode all signals from the golden frame
        let mut signal_values: Vec<(&str, f64)> = Vec::new();
        for signal in msg.signals() {
            let layout = SignalLayout::from_spec(signal);
            let value = layout.decode(&golden_frame, signal);
            signal_values.push((signal.name(), value));
        }

        // Re-encode from the decoded physical values
        let encoded = encode_message(msg, &signal_values, 0x1F0).unwrap();

        // Decode from the re-encoded frame and verify signal values match
        for signal in msg.signals() {
            let layout = SignalLayout::from_spec(signal);
            let golden_value = layout.decode(&golden_frame, signal);
            let encoded_value = layout.decode(&encoded, signal);
            assert!(
                (golden_value - encoded_value).abs() < 1e-9,
                "signal '{}': golden={}, re-encoded={}",
                signal.name(), golden_value, encoded_value
            );
        }
    }

    #[test]
    fn test_encode_reproduces_golden_frame_signed() {
        // For each signed.dbc message: decode golden frame, re-encode,
        // verify all signal values match.
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();

        let frames_and_messages: &[(&str, &str)] = &[
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910"),
            ("(0.0) vcan0 002#11223344FF667788", "Message64"),
            ("(0.0) vcan0 003#8000000000000000", "Message64big"),
            ("(0.0) vcan0 000#11223344FF667788", "Message32"),
            ("(0.0) vcan0 005#11223344FF667788", "Message32big"),
            ("(0.0) vcan0 001#11223344FF667788", "Message33"),
            ("(0.0) vcan0 004#11223344FF667788", "Message33big"),
            ("(0.0) vcan0 006#11223344FF667788", "Message63"),
            ("(0.0) vcan0 007#11223344FF667788", "Message63big"),
        ];

        for (line, msg_name) in frames_and_messages {
            let golden_frame = canlog_reader::parse_candump_line(line).unwrap();
            let msg = match can_decoder::get_message_spec(&dbc, msg_name) {
                Some(m) => m,
                None => continue,
            };

            // Decode all signals, then re-encode
            let mut signal_values: Vec<(&str, f64)> = Vec::new();
            for signal in msg.signals() {
                let layout = SignalLayout::from_spec(signal);
                let value = layout.decode(&golden_frame, signal);
                signal_values.push((signal.name(), value));
            }

            let encoded = encode_message(msg, &signal_values, golden_frame.id).unwrap();

            // Verify each signal decodes to the same value from both frames
            for signal in msg.signals() {
                let layout = SignalLayout::from_spec(signal);
                let golden_value = layout.decode(&golden_frame, signal);
                let encoded_value = layout.decode(&encoded, signal);
                assert_eq!(
                    golden_value, encoded_value,
                    "signal {}.{}: golden={}, re-encoded={}",
                    msg_name, signal.name(), golden_value, encoded_value
                );
            }
        }
    }
}
