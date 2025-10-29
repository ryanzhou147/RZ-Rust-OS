#![no_std]
extern crate alloc;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packetbuf_basic() {
        let mut p = PacketBuf::with_capacity(128);
        p.push_bytes(&[1,2,3]);
        assert_eq!(p.len(), 3);
    }
}
