extern crate alloc;
use core::fmt;
use core::fmt::Write as FmtWrite;
use alloc::vec::Vec;
use alloc::string::String;

/// ARP packet format constants
pub const ARP_HDR_LEN: usize = 28; // ARP payload length for Ethernet/IPv4

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
    let mut s = String::new();
    for (i, b) in m.iter().enumerate() {
        if i != 0 { let _ = write!(s, ":"); }
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn format_ip(ip: &[u8;4]) -> String {
    let mut s = String::new();
    let _ = write!(s, "{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
    s
}

/// Fixed-size ARP cache (small). Simple linear scan and LRU by age counter.
pub const ARP_CACHE_CAPACITY: usize = 16;
pub struct ArpCache {
    entries: [ArpEntry; ARP_CACHE_CAPACITY],
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
    pub fn new() -> Self {
        ArpCache { entries: [ArpEntry::default(); ARP_CACHE_CAPACITY], clock: 1 }
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
                e.ip = ip; e.mac = mac; e.age = self.clock; e.valid = true; return;
            }
        }
        // no free slot: evict LRU
        let mut lru_idx = 0usize;
        let mut lru_age = u64::MAX;
        for (i, e) in self.entries.iter().enumerate() {
            if e.age < lru_age {
                lru_age = e.age; lru_idx = i;
            }
        }
        self.entries[lru_idx] = ArpEntry { ip, mac, age: self.clock, valid: true };
    }

    /// Remove a mapping (if present)
    pub fn remove(&mut self, ip: [u8;4]) {
        for e in self.entries.iter_mut() {
            if e.valid && e.ip == ip { e.valid = false; }
        }
    }
}

/// Parse an ARP payload (28 bytes) into an ArpPacket
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

/// Build an ARP reply payload (28 bytes) given a parsed request and our addresses.
pub fn build_arp_reply(req: &ArpPacket, my_mac: [u8;6], my_ip: [u8;4]) -> Vec<u8> {
    let mut out = Vec::with_capacity(ARP_HDR_LEN);
    out.extend_from_slice(&1u16.to_be_bytes()); // htype = Ethernet
    out.extend_from_slice(&0x0800u16.to_be_bytes()); // ptype = IPv4
    out.push(6); out.push(4); // hlen, plen
    out.extend_from_slice(&2u16.to_be_bytes()); // opcode = reply
    out.extend_from_slice(&my_mac); // sender hw
    out.extend_from_slice(&my_ip); // sender proto
    out.extend_from_slice(&req.sender_mac); // target hw = requester
    out.extend_from_slice(&req.sender_ip); // target proto = requester ip
    out
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
    // hardware type (2), protocol type (2),
    // hardware address length (1), protocol address length (1), operation code (2)
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