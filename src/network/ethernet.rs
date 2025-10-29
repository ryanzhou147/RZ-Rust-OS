pub const ETH_HEADER_LEN: usize = 14;
pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16  = 0x0806;

pub struct EthHeader {
    pub dst: [u8;6],
    pub src: [u8;6],
    pub ethertype: u16,
}

/// Parse eth header from buffer -> returns (EthHeader, payload_slice)
pub fn parse_eth_header(_buf: &[u8]) -> Option<(EthHeader, &[u8])> {
    // TODO: implement parsing
    None
}

/// Serialize header + payload into frame (caller provides buffer)
pub fn build_eth_frame(_dst: [u8;6], _src: [u8;6], _ethertype: u16, _payload: &[u8], _out: &mut [u8]) -> Option<usize> {
    // TODO: implement serialization
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compile_parse() {
        let b = [0u8; 64];
        let _ = parse_eth_header(&b);
    }
}
