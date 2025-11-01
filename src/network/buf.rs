use alloc::vec::Vec;

/// Simple packet buffer backed by Vec<u8>.
pub struct PacketBuf {
    data: Vec<u8>,
}

impl PacketBuf {
    pub fn with_capacity(cap: usize) -> Self {
        PacketBuf { data: Vec::with_capacity(cap) }
    }
    pub fn len(&self) -> usize { self.data.len() }
    pub fn as_slice(&self) -> &[u8] { &self.data }
    pub fn push_bytes(&mut self, b: &[u8]) { self.data.extend_from_slice(b); }
}

