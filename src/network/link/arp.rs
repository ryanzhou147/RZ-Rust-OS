extern crate alloc;
use alloc::vec::Vec;

/// Simple ARP cache skeleton
pub struct ArpCache {
    // TODO: storage for IP -> MAC mappings
}

impl ArpCache {
    pub fn new() -> Self { ArpCache {} }

    /// Lookup a MAC for an IPv4 address. Returns Some(mac) or None.
    pub fn lookup(&self, _ip: [u8;4]) -> Option<[u8;6]> { None }

    /// Insert mapping
    pub fn insert(&mut self, _ip: [u8;4], _mac: [u8;6]) { /* TODO */ }
}

/// Handle an incoming ARP packet; may return an ARP reply frame to transmit.
pub fn handle_arp_packet(_frame: &[u8]) -> Option<Vec<u8>> {
    // TODO: parse and optionally build reply
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arp_cache_basic() {
        let mut c = ArpCache::new();
        assert!(c.lookup([0,0,0,0]).is_none());
        c.insert([1,2,3,4], [5,6,7,8,9,10]);
    }
}
