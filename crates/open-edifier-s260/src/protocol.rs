use serde_json::Value;

use crate::Result;

/// Four-byte header that starts an S260 response frame.
pub(crate) const FRAME_HEADER: [u8; 4] = [0xee, 0xdd, 0xff, 0xee];

/// Incremental decoder for framed S260 responses and interleaved heartbeats.
#[derive(Debug, Default)]
pub(crate) struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    /// Creates a decoder for the verified plaintext S260 transport.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Feeds a possibly fragmented byte chunk and returns complete JSON messages.
    pub(crate) fn feed(&mut self, chunk: &[u8]) -> Vec<Result<Value>> {
        self.buffer.extend_from_slice(chunk);

        let mut messages = Vec::new();
        loop {
            let Some(marker) = self
                .buffer
                .windows(FRAME_HEADER.len())
                .position(|window| window == FRAME_HEADER)
            else {
                let keep = self.buffer.len().min(FRAME_HEADER.len() - 1);
                self.buffer.drain(..self.buffer.len() - keep);
                break;
            };

            if marker > 0 {
                self.buffer.drain(..marker);
            }
            if self.buffer.len() < 6 {
                break;
            }

            let length = u16::from_be_bytes([self.buffer[4], self.buffer[5]]) as usize;
            let frame_end = 6 + length;
            if self.buffer.len() < frame_end {
                break;
            }

            let payload = self.buffer[6..frame_end].to_vec();
            self.buffer.drain(..frame_end);
            messages.push(serde_json::from_slice(&payload).map_err(Into::into));
        }
        messages
    }
}

pub(crate) fn encode_request(value: &Value) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(value: &Value, prefix: &[u8]) -> Vec<u8> {
        let payload = serde_json::to_vec(value).unwrap();
        let mut result = prefix.to_vec();
        result.extend(FRAME_HEADER);
        result.extend((payload.len() as u16).to_be_bytes());
        result.extend(payload);
        result
    }

    fn binary_frame(command: u16, payload: &[u8]) -> Vec<u8> {
        let mut result = vec![0xbb, 0xec];
        result.extend(command.to_le_bytes());
        result.push(payload.len() as u8);
        result.extend(payload);
        let checksum = result
            .iter()
            .fold(0_u8, |sum, byte| sum.wrapping_add(*byte));
        result.push(checksum);
        result
    }

    #[test]
    fn decodes_fragmented_frame_after_heartbeat() {
        let expected = serde_json::json!({"id":"1","payload":"status_query"});
        let data = frame(&expected, &binary_frame(0x003f, &[0; 9]));
        let mut decoder = FrameDecoder::new();
        assert!(decoder.feed(&data[..7]).is_empty());
        let decoded: Vec<_> = decoder
            .feed(&data[7..])
            .into_iter()
            .collect::<Result<_>>()
            .unwrap();
        assert_eq!(decoded, vec![expected]);
    }

    #[test]
    fn recovers_after_malformed_json_in_the_same_chunk() {
        let expected = serde_json::json!({"id":"2","payload":"status_query"});
        let mut malformed = Vec::from(FRAME_HEADER);
        malformed.extend(1_u16.to_be_bytes());
        malformed.push(b'{');
        malformed.extend(frame(&expected, &[]));

        let decoded = FrameDecoder::new().feed(&malformed);
        assert!(decoded[0].is_err());
        assert_eq!(decoded[1].as_ref().unwrap(), &expected);
    }
}
