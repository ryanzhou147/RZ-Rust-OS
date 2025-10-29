extern crate alloc;
use alloc::vec::Vec;

use crate::network::ipv4::Ipv4Header;

/// Handle ICMP packet; optionally return a reply payload (to be wrapped in IPv4+ETH by caller)
pub fn handle_icmp(_hdr: &Ipv4Header, _payload: &[u8]) -> Option<Vec<u8>> {
    // TODO: if it's an echo request, build echo reply
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::ipv4::Ipv4Header;
    #[test]
    fn icmp_compile() {
        let hdr = Ipv4Header { src: [0,0,0,0], dst: [0,0,0,0], proto: 1, header_len: 5, total_len: 20 };
        let _ = handle_icmp(&hdr, &[]);
    }
}
