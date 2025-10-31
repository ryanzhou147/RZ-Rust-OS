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
    if _buf.len() < ETH_HEADER_LEN { return None; }

    let mut dst = [0u8;6];
    let mut src = [0u8;6];
    dst.copy_from_slice(&_buf[0..6]);
    src.copy_from_slice(&_buf[6..12]);
    let ethertype = u16::from_be_bytes([_buf[12], _buf[13]]);
    let header = EthHeader { dst, src, ethertype };
    let payload = &_buf[ETH_HEADER_LEN..];
    Some((header, payload))
}

/// Serialize header + payload into frame (caller provides buffer)
pub fn build_eth_frame(_dst: [u8;6], _src: [u8;6], _ethertype: u16, _payload: &[u8], _out: &mut [u8]) -> Option<usize> {
    let total_len = ETH_HEADER_LEN + _payload.len();
    if _out.len() < total_len { return None; }

    _out[0..6].copy_from_slice(&_dst);
    _out[6..12].copy_from_slice(&_src);
    let et = _ethertype.to_be_bytes();
    _out[12] = et[0];
    _out[13] = et[1];
    _out[ETH_HEADER_LEN..total_len].copy_from_slice(_payload);
    Some(total_len)
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
