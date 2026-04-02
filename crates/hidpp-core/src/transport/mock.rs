use std::collections::VecDeque;
use std::sync::Mutex;

use super::HidTransport;
use crate::protocol::error::HidppError;

/// Deterministic in-memory transport for unit tests.
/// Caller pre-loads response bytes; writes are recorded for inspection.
pub struct MockTransport {
    responses: Mutex<VecDeque<Vec<u8>>>,
    writes: Mutex<Vec<Vec<u8>>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(VecDeque::new()),
            writes: Mutex::new(Vec::new()),
        }
    }

    /// Queue a response that will be returned by the next read() call.
    pub fn push_response(&self, response: Vec<u8>) {
        self.responses.lock().unwrap().push_back(response);
    }

    /// Returns all data that was passed to write() calls, in order.
    pub fn written_data(&self) -> Vec<Vec<u8>> {
        self.writes.lock().unwrap().clone()
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HidTransport for MockTransport {
    fn write(&self, data: &[u8]) -> Result<(), HidppError> {
        self.writes.lock().unwrap().push(data.to_vec());
        Ok(())
    }

    fn read(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize, HidppError> {
        let mut responses = self.responses.lock().unwrap();
        match responses.pop_front() {
            Some(resp) => {
                let n = resp.len().min(buf.len());
                buf[..n].copy_from_slice(&resp[..n]);
                Ok(n)
            }
            None => Err(HidppError::Timeout { timeout_ms: 0 }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_write_and_read() {
        let transport = MockTransport::new();
        transport.push_response(vec![0x10, 0xFF, 0x01, 0x00, 0x06, 0x40, 0x00]);

        transport
            .write(&[0x00, 0x10, 0xFF, 0x01, 0x20, 0x00, 0x00, 0x00])
            .unwrap();

        let mut buf = [0u8; 32];
        let n = transport.read(&mut buf, 1000).unwrap();
        assert_eq!(n, 7);
        assert_eq!(buf[0], 0x10); // SHORT_REPORT_ID

        let writes = transport.written_data();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0][0], 0x00); // leading report ID prefix
    }
}
