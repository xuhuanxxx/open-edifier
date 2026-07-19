//! Safe framing primitives for the EDIFIER `AA EC` request and `BB EC` response protocol.
#![warn(missing_docs)]

/// Magic prefix used by client request frames.
pub const REQUEST_MAGIC: [u8; 2] = [0xaa, 0xec];
/// Magic prefix used by speaker response and event frames.
pub const RESPONSE_MAGIC: [u8; 2] = [0xbb, 0xec];
/// Command used by the module heartbeat observed on supported devices.
pub const HEARTBEAT_COMMAND: u16 = 0x003f;

/// A decoded binary response or event frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Little-endian command identifier.
    pub command: u16,
    /// Command-specific payload bytes.
    pub payload: Vec<u8>,
}

impl Frame {
    /// Returns whether this frame is a module heartbeat.
    pub fn is_heartbeat(&self) -> bool {
        self.command == HEARTBEAT_COMMAND
    }
}

/// Incremental decoder for fragmented `BB EC` frames and interleaved garbage.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    /// Creates an empty decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds bytes and returns every complete, checksum-validated frame.
    ///
    /// Corrupt candidates are skipped one byte at a time so a later valid frame
    /// in the same stream can still be recovered.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<Frame> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();

        loop {
            let Some(marker) = self
                .buffer
                .windows(RESPONSE_MAGIC.len())
                .position(|window| window == RESPONSE_MAGIC)
            else {
                let keep = self.buffer.len().min(RESPONSE_MAGIC.len() - 1);
                self.buffer.drain(..self.buffer.len() - keep);
                break;
            };
            if marker > 0 {
                self.buffer.drain(..marker);
            }
            if self.buffer.len() < 6 {
                break;
            }

            let payload_length = usize::from(self.buffer[4]);
            let frame_end = 5 + payload_length + 1;
            if self.buffer.len() < frame_end {
                break;
            }
            let expected = checksum(&self.buffer[..frame_end - 1]);
            let actual = self.buffer[frame_end - 1];
            if expected != actual {
                self.buffer.drain(..1);
                continue;
            }

            frames.push(Frame {
                command: u16::from_le_bytes([self.buffer[2], self.buffer[3]]),
                payload: self.buffer[5..5 + payload_length].to_vec(),
            });
            self.buffer.drain(..frame_end);
        }
        frames
    }
}

/// Encodes a checksum-protected `AA EC` request frame.
pub fn encode_request(command: u16, payload: &[u8]) -> Result<Vec<u8>> {
    let payload_length =
        u8::try_from(payload.len()).map_err(|_| Error::PayloadTooLarge(payload.len()))?;
    let mut frame = Vec::with_capacity(6 + payload.len());
    frame.extend(REQUEST_MAGIC);
    frame.extend(command.to_le_bytes());
    frame.push(payload_length);
    frame.extend(payload);
    frame.push(checksum(&frame));
    Ok(frame)
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(0_u8, |sum, value| sum.wrapping_add(*value))
}

/// Binary framing failure.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Payload cannot fit in the protocol's one-byte length field.
    #[error("payload has {0} bytes; AA EC frames support at most 255")]
    PayloadTooLarge(usize),
}

/// Convenience result type for binary framing.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    fn response(command: u16, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::from(RESPONSE_MAGIC);
        frame.extend(command.to_le_bytes());
        frame.push(payload.len() as u8);
        frame.extend(payload);
        frame.push(checksum(&frame));
        frame
    }

    #[test]
    fn encodes_verified_one_byte_length_request() {
        assert_eq!(
            encode_request(0x0061, &[]).unwrap(),
            vec![0xaa, 0xec, 0x61, 0x00, 0x00, 0xf7]
        );
    }

    #[test]
    fn decodes_fragmented_event_after_unrelated_bytes() {
        let wire = response(0x0066, &[30, 18]);
        let mut decoder = FrameDecoder::new();
        assert!(decoder.feed(&[0xee, 0xdd, 0xff, 0xee]).is_empty());
        assert!(decoder.feed(&wire[..3]).is_empty());
        assert_eq!(
            decoder.feed(&wire[3..]),
            vec![Frame {
                command: 0x0066,
                payload: vec![30, 18]
            }]
        );
    }

    #[test]
    fn identifies_heartbeat_and_recovers_after_bad_checksum() {
        let mut heartbeat = response(HEARTBEAT_COMMAND, &[0; 9]);
        assert!(FrameDecoder::new().feed(&heartbeat)[0].is_heartbeat());
        let last = heartbeat.len() - 1;
        heartbeat[last] ^= 0xff;
        let expected = response(0x0066, &[30, 18]);
        heartbeat.extend(&expected);
        assert_eq!(
            FrameDecoder::new().feed(&heartbeat),
            vec![Frame {
                command: 0x0066,
                payload: vec![30, 18],
            }]
        );
    }
}
