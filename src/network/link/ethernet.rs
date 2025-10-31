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
    if _buf.len() < ETH_HEADER_LEN {
        return None;
    }
    let mut dst = [0u8;6];
    let mut src = [0u8;6];
    dst.copy_from_slice(&_buf[0..6]);
    src.copy_from_slice(&_buf[6..12]);
    let ethertype = u16::from_be_bytes([_buf[12], _buf[13]]);
    let payload = &_buf[ETH_HEADER_LEN..];
    Some((EthHeader { dst, src, ethertype }, payload))
}

/// Serialize header + payload into frame (caller provides buffer)
pub fn build_eth_frame(_dst: [u8;6], _src: [u8;6], _ethertype: u16, _payload: &[u8], _out: &mut [u8]) -> Option<usize> {
    let needed = ETH_HEADER_LEN + _payload.len();
    if _out.len() < needed {
        return None;
    }
    _out[0..6].copy_from_slice(&_dst);
    _out[6..12].copy_from_slice(&_src);
    let et = _ethertype.to_be_bytes();
    _out[12] = et[0];
    _out[13] = et[1];
    _out[14..14 + _payload.len()].copy_from_slice(_payload);
    Some(needed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compile_parse() {
        let b = [0u8; 64];
        let _ = parse_eth_header(&b);
    }

    #[test]
    fn build_and_parse_roundtrip() {
        let dst = [1u8,2,3,4,5,6];
        let src = [10u8,11,12,13,14,15];
        let ethertype = ETHERTYPE_IPV4;
        let payload = [0x45u8, 0, 0x00, 0x54];
        let mut out = [0u8; 128];
        let len = build_eth_frame(dst, src, ethertype, &payload, &mut out).expect("build failed");
        let (hdr, pl) = parse_eth_header(&out[..len]).expect("parse failed");
        assert_eq!(hdr.dst, dst);
        assert_eq!(hdr.src, src);
        assert_eq!(hdr.ethertype, ethertype);
        assert_eq!(pl, &payload);
    }
}
