extern crate alloc;
use alloc::vec::Vec;

use crate::network::ipv4::Ipv4Header;
use crate::network::checksums::compute_ck16;

/// Handle ICMP packet; optionally return a reply payload (to be wrapped in IPv4+ETH by caller)
pub fn handle_icmp(_hdr: &Ipv4Header, _payload: &[u8]) -> Option<Vec<u8>> {
    // ICMP header for Echo: Type(1), Code(1), Checksum(2), Identifier(2), Sequence(2), Data(...)
    if _payload.len() < 8 {
        return None;
    }
    let icmp_type = _payload[0];
    let _code = _payload[1];
    let recv_ck = u16::from_be_bytes([_payload[2], _payload[3]]);

    // Only handle Echo Request (type 8)
    const ICMP_ECHO_REQUEST: u8 = 8;
    const ICMP_ECHO_REPLY: u8 = 0;

    if icmp_type != ICMP_ECHO_REQUEST {
        return None;
    }

    // Validate checksum: recompute with checksum field zeroed
    let mut tmp_in = _payload.to_vec();
    tmp_in[2] = 0; tmp_in[3] = 0;
    let computed = compute_ck16(&tmp_in);
    if computed != recv_ck {
        return None;
    }

    // Build reply: Type=0 Code=0, checksum filled later, copy identifier/seq and payload
    let mut out: Vec<u8> = Vec::with_capacity(_payload.len());
    out.push(ICMP_ECHO_REPLY);
    out.push(0u8); // code
    out.extend_from_slice(&[0u8, 0u8]); // checksum placeholder
    // identifier + sequence (bytes 4..8)
    out.extend_from_slice(&_payload[4..8]);
    // data
    if _payload.len() > 8 {
        out.extend_from_slice(&_payload[8..]);
    }

    // compute checksum and write it
    let chk = compute_ck16(&out);
    let chk_be = chk.to_be_bytes();
    out[2] = chk_be[0]; out[3] = chk_be[1];
    Some(out)
}