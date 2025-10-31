use core::fmt;

/// ARP packet format constants
pub const ARP_HDR_LEN: usize = 28; // Ethernet + IPv4

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpOp {
    Request = 1,
    Reply = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArpPacket {
    pub htype: u16,
    pub ptype: u16,
    pub hlen: u8,
    pub plen: u8,
    pub opcode: u16,
    pub sender_mac: [u8;6],
    pub sender_ip: [u8;4],
    pub target_mac: [u8;6],
    pub target_ip: [u8;4],
}

impl fmt::Display for ArpPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ARP op={} sender={} {} target={} {}",
               self.opcode,
               hex_mac(&self.sender_mac),
               format_ip(&self.sender_ip),
               hex_mac(&self.target_mac),
               format_ip(&self.target_ip))
    }
}

fn hex_mac(m: &[u8;6]) -> String {
    use alloc::string::String;
    use alloc::fmt::Write;
    let mut s = String::new();
    for (i, b) in m.iter().enumerate() {
        if i != 0 { let _ = write!(s, ":"); }
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn format_ip(ip: &[u8;4]) -> String {
    use alloc::string::String;
    use alloc::fmt::Write;
    let mut s = String::new();
    let _ = write!(s, "{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
    s
}

/// Fixed-size ARP cache (small, no-alloc). Simple linear scan and LRU by age counter.
pub struct ArpCache {
    entries: [ArpEntry; ArpCache::CAPACITY],
    clock: u64,
}

#[derive(Clone, Copy)]
struct ArpEntry {
    ip: [u8;4],
    mac: [u8;6],
    age: u64,
    valid: bool,
}

impl Default for ArpEntry {
    fn default() -> Self {
        ArpEntry { ip: [0;4], mac: [0;6], age: 0, valid: false }
    }
}

impl ArpCache {
    const CAPACITY: usize = 16;

    pub fn new() -> Self {
        ArpCache { entries: [ArpEntry::default(); ArpCache::CAPACITY], clock: 1 }
    }

    /// Lookup a MAC for an IPv4 address. Returns Some(mac) or None.
    pub fn lookup(&self, ip: [u8;4]) -> Option<[u8;6]> {
        for e in self.entries.iter() {
            if e.valid && e.ip == ip { return Some(e.mac); }
        }
        None
    }

    /// Insert mapping (overwrites LRU or empty slot)
    pub fn insert(&mut self, ip: [u8;4], mac: [u8;6]) {
        // update clock
        self.clock = self.clock.wrapping_add(1);
        // if already present, update
        for e in self.entries.iter_mut() {
            if e.valid && e.ip == ip {
                e.mac = mac;
                e.age = self.clock;
                return;
            }
        }
        // find invalid entry
        for e in self.entries.iter_mut() {
            if !e.valid {
                e.valid = true; e.ip = ip; e.mac = mac; e.age = self.clock; return;
            }
        }
        // replace LRU
        let mut lru_idx = 0usize;
        let mut min_age = u64::MAX;
        for (i, e) in self.entries.iter().enumerate() {
            if e.age < min_age { min_age = e.age; lru_idx = i; }
        }
        let e = &mut self.entries[lru_idx];
        e.ip = ip; e.mac = mac; e.age = self.clock; e.valid = true;
    }
}

/// Parse an ARP packet payload (28 bytes minimum). Returns ArpPacket or None.
pub fn parse_arp_packet(buf: &[u8]) -> Option<ArpPacket> {
    if buf.len() < ARP_HDR_LEN { return None; }
    let htype = u16::from_be_bytes([buf[0], buf[1]]);
    let ptype = u16::from_be_bytes([buf[2], buf[3]]);
    let hlen = buf[4];
    let plen = buf[5];
    let opcode = u16::from_be_bytes([buf[6], buf[7]]);
    if hlen as usize != 6 || plen as usize != 4 { return None; }
    let mut sender_mac = [0u8;6]; sender_mac.copy_from_slice(&buf[8..14]);
    let mut sender_ip = [0u8;4]; sender_ip.copy_from_slice(&buf[14..18]);
    let mut target_mac = [0u8;6]; target_mac.copy_from_slice(&buf[18..24]);
    let mut target_ip = [0u8;4]; target_ip.copy_from_slice(&buf[24..28]);
    Some(ArpPacket { htype, ptype, hlen, plen, opcode, sender_mac, sender_ip, target_mac, target_ip })
}

/// Build an ARP reply payload (not Ethernet frame) given the request's fields and our MAC/IP.
pub fn build_arp_reply(req: &ArpPacket, our_mac: [u8;6], our_ip: [u8;4]) -> Vec<u8> {
    use alloc::vec::Vec;
    let mut out = Vec::with_capacity(ARP_HDR_LEN);
    // htype
    out.extend_from_slice(&req.htype.to_be_bytes());
    out.extend_from_slice(&req.ptype.to_be_bytes());
    out.push(6u8); // hlen
    out.push(4u8); // plen
    out.extend_from_slice(&(ArpOp::Reply as u16).to_be_bytes());
    // sender mac/ip (our)
    out.extend_from_slice(&our_mac);
    out.extend_from_slice(&our_ip);
    // target mac/ip (original sender)
    out.extend_from_slice(&req.sender_mac);
    out.extend_from_slice(&req.sender_ip);
    out
}

/// Handle an incoming ARP payload (Ethernet payload). This function parses the packet
/// and returns an ARP reply payload if one should be sent. The caller is responsible
/// for wrapping it in an Ethernet frame to transmit. At present this function does
/// NOT know the device's IP/MAC, so it only parses and returns None. Use `parse_arp_packet`
/// and `build_arp_reply` directly when the caller has device addresses.
pub fn handle_arp_packet(_frame: &[u8]) -> Option<Vec<u8>> {
    // Keep API stable; for now parse and ignore.
    let _ = parse_arp_packet(_frame);
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
    }

    #[test]
    fn parse_and_build() {
        // build a request packet
        let mut req = vec![];
        req.extend_from_slice(&1u16.to_be_bytes()); // htype
        req.extend_from_slice(&0x0800u16.to_be_bytes()); // ptype IPv4
        req.push(6); req.push(4);
        req.extend_from_slice(&1u16.to_be_bytes()); // request
        let sender_mac = [1u8,2,3,4,5,6];
        let sender_ip = [10u8,0,0,1];
        let target_mac = [0u8;6];
        let target_ip = [10u8,0,0,2];
        req.extend_from_slice(&sender_mac);
        req.extend_from_slice(&sender_ip);
        req.extend_from_slice(&target_mac);
        req.extend_from_slice(&target_ip);
        let pkt = parse_arp_packet(&req).expect("parse");
        assert_eq!(pkt.sender_ip, sender_ip);
        let reply = build_arp_reply(&pkt, [9u8,9,9,9,9,9], [10u8,0,0,2]);
        let parsed = parse_arp_packet(&reply).expect("parse reply");
        assert_eq!(parsed.opcode, ArpOp::Reply as u16);
        assert_eq!(parsed.sender_ip, [10u8,0,0,2]);
        assert_eq!(parsed.target_ip, sender_ip);
    }
}
