extern crate alloc;
use alloc::vec::Vec;
/// Small ARP cache with linear storage (suitable for skeleton/testing).
pub struct ArpCache {
    entries: Vec<ArpEntry>,
}

#[derive(Clone, Copy)]
struct ArpEntry {
    ip: [u8;4],
    mac: [u8;6],
}

impl ArpCache {
    /// Create an empty ARP cache.
    pub fn new() -> Self { ArpCache { entries: Vec::new() } }

    /// Lookup a MAC for an IPv4 address. Returns Some(mac) or None.
    pub fn lookup(&self, ip: [u8;4]) -> Option<[u8;6]> {
        for e in self.entries.iter() {
            if e.ip == ip { return Some(e.mac); }
        }
        None
    }

    /// Insert or update mapping
    pub fn insert(&mut self, ip: [u8;4], mac: [u8;6]) {
        for e in self.entries.iter_mut() {
            if e.ip == ip { e.mac = mac; return; }
        }
        self.entries.push(ArpEntry { ip, mac });
    }

    /// Remove a mapping (if present)
    pub fn remove(&mut self, ip: [u8;4]) {
        self.entries.retain(|e| e.ip != ip);
    }
}

/// Parse an incoming ARP Ethernet frame and optionally build an ARP reply
/// if `our_ip` and `our_mac` are provided and the packet is an ARP request
/// directed at `our_ip`.
///
/// Returns Some(frame_bytes) containing a full Ethernet frame (eth header + arp)
/// to transmit, or None if no reply should be sent or parse failed.
pub fn handle_arp_packet(frame: &[u8], our_ip: Option<[u8;4]>, our_mac: Option<[u8;6]>) -> Option<Vec<u8>> {
    // Minimum Ethernet + ARP packet size: 14 + 28 = 42
    if frame.len() < 42 { return None; }

    // Ethernet header
    let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
    if ethertype != 0x0806 { return None; } // not ARP

    // ARP payload starts at offset 14
    let arp = &frame[14..];
    // hardware type (2), proto type (2), hlen (1), plen (1), opcode (2)
    let opcode = u16::from_be_bytes([arp[6], arp[7]]);
    let sender_hw = &arp[8..14];
    let sender_proto = &arp[14..18];
    let target_proto = &arp[24..28];

    // Only handle Ethernet/IPv4 ARP (htype=1, ptype=0x0800)
    if arp.len() < 28 { return None; }

    if opcode == 1 {
        // ARP Request
        if let (Some(my_ip), Some(my_mac)) = (our_ip, our_mac) {
            if target_proto == &my_ip {
                // build ARP reply frame
                let mut out: Vec<u8> = Vec::with_capacity(14 + 28);
                // ethernet dst = sender_hw, src = my_mac, ethertype = 0x0806
                out.extend_from_slice(sender_hw); // dst
                out.extend_from_slice(&my_mac); // src
                out.extend_from_slice(&0x0806u16.to_be_bytes()); // ethertype
                // ARP payload
                out.extend_from_slice(&1u16.to_be_bytes()); // htype = Ethernet
                out.extend_from_slice(&0x0800u16.to_be_bytes()); // ptype = IPv4
                out.push(6u8); // hlen
                out.push(4u8); // plen
                out.extend_from_slice(&2u16.to_be_bytes()); // opcode = reply
                // sender hw addr (our_mac)
                out.extend_from_slice(&my_mac);
                // sender proto (our_ip)
                out.extend_from_slice(&my_ip);
                // target hw = sender_hw
                out.extend_from_slice(sender_hw);
                // target proto = sender_proto
                out.extend_from_slice(sender_proto);
                return Some(out);
            }
        }
    }
    // For ARP reply or other opcodes we do not craft responses in skeleton.
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
        assert_eq!(c.lookup([1,2,3,4]).unwrap(), [5,6,7,8,9,10]);
        c.remove([1,2,3,4]);
        assert!(c.lookup([1,2,3,4]).is_none());
    }

    #[test]
    fn handle_request_builds_reply() {
        // craft a minimal ARP request for target 10.0.0.2 from sender 10.0.0.1
        let sender_mac = [0x02,0x00,0x00,0x00,0x00,0x01];
        let sender_ip = [10,0,0,1];
        let target_ip = [10,0,0,2];
        let mut req: Vec<u8> = Vec::new();
        // ethernet dst (broadcast)
        req.extend_from_slice(&[0xff;6]);
        // ethernet src
        req.extend_from_slice(&sender_mac);
        // ethertype ARP
        req.extend_from_slice(&0x0806u16.to_be_bytes());
        // ARP payload
        req.extend_from_slice(&1u16.to_be_bytes()); // htype
        req.extend_from_slice(&0x0800u16.to_be_bytes()); // ptype
        req.push(6u8); // hlen
        req.push(4u8); // plen
        req.extend_from_slice(&1u16.to_be_bytes()); // opcode = request
        req.extend_from_slice(&sender_mac); // sender hw
        req.extend_from_slice(&sender_ip); // sender proto
        req.extend_from_slice(&[0u8;6]); // target hw (unknown)
        req.extend_from_slice(&target_ip); // target proto

        let our_mac = [0x02,0x00,0x00,0x00,0x00,0x02];
        let reply = handle_arp_packet(&req, Some(target_ip), Some(our_mac));
        assert!(reply.is_some());
        let r = reply.unwrap();
        // basic sanity checks: ethertype ARP at bytes 12..14
        assert_eq!(&r[12..14], &0x0806u16.to_be_bytes());
        // opcode in ARP payload should be 2 (reply)
        assert_eq!(&r[14+6..14+8], &2u16.to_be_bytes());
        // reply sender hw should be our_mac
        assert_eq!(&r[14+8..14+14], &our_mac);
    }
}
