extern crate alloc;
use alloc::vec::Vec;

pub struct Ipv4Header {
    pub src: [u8;4],
    pub dst: [u8;4],
    pub proto: u8,
    pub header_len: u8,
    pub total_len: u16,
}

pub fn parse_ipv4_header(_buf: &[u8]) -> Option<(Ipv4Header, &[u8])> {
    // TODO: implement parsing
    None
}

pub fn build_ipv4_packet(_src: [u8;4], _dst: [u8;4], _proto: u8, _payload: &[u8], _out: &mut [u8]) -> Option<usize> {
    // TODO: implement packet building (set checksum, etc.)
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ipv4_compile() {
        let b = [0u8; 64];
        let _ = parse_ipv4_header(&b);
    }
}
