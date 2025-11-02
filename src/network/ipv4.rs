extern crate alloc;
use crate::network::checksums::ipv4_checksum;

pub struct Ipv4Header {
    pub src: [u8;4],
    pub dst: [u8;4],
    pub proto: u8,
    pub header_len: u8,
    pub total_len: u16,
}

pub fn parse_ipv4_header(_buf: &[u8]) -> Option<(Ipv4Header, &[u8])> {
    if _buf.len() < 20 {
        return None;
    }
    let ver_ihl = _buf[0];
    let version = ver_ihl >> 4;
    if version != 4 { return None; }
    let ihl = ver_ihl & 0x0f; // in 32-bit words
    let header_len = (ihl as usize) * 4;
    if header_len < 20 || _buf.len() < header_len { return None; }
    let total_len = u16::from_be_bytes([_buf[2], _buf[3]]);
    if (_buf.len() as u16) < total_len { return None; }
    let proto = _buf[9];
    let mut src = [0u8;4]; src.copy_from_slice(&_buf[12..16]);
    let mut dst = [0u8;4]; dst.copy_from_slice(&_buf[16..20]);

    // Validate checksum: the header checksum field should equal computed checksum
    let hdr = &_buf[..header_len];
    let computed = ipv4_checksum(hdr);
    let hdr_ck = u16::from_be_bytes([hdr[10], hdr[11]]);
    if hdr_ck != computed { return None; }

    Some((Ipv4Header { src, dst, proto, header_len: header_len as u8, total_len }, &_buf[header_len..(total_len as usize)]))
}

pub fn build_ipv4_packet(_src: [u8;4], _dst: [u8;4], _proto: u8, _payload: &[u8], _out: &mut [u8]) -> Option<usize> {
    // Minimal IPv4 header without options (IHL=5 -> 20 bytes)
    let header_len = 20usize;
    let total_len = header_len + _payload.len();
    if total_len > 0xffff { return None; }
    if _out.len() < total_len { return None; }

    // version(4) + IHL(5)
    _out[0] = (4u8 << 4) | 5u8;
    // DSCP/ECN
    _out[1] = 0;
    // total length
    let tot_be = (total_len as u16).to_be_bytes();
    _out[2] = tot_be[0]; _out[3] = tot_be[1];
    // identification
    _out[4] = 0; _out[5] = 0;
    // flags + fragment offset
    _out[6] = 0; _out[7] = 0;
    // TTL
    _out[8] = 64;
    // protocol
    _out[9] = _proto;
    // checksum placeholder
    _out[10] = 0; _out[11] = 0;
    // src/dst
    _out[12..16].copy_from_slice(&_src);
    _out[16..20].copy_from_slice(&_dst);

    // payload
    _out[20..20+_payload.len()].copy_from_slice(_payload);

    // compute checksum over header
    let sum = ipv4_checksum(&_out[..header_len]);
    let sum_be = sum.to_be_bytes();
    _out[10] = sum_be[0]; _out[11] = sum_be[1];

    Some(total_len)
}



