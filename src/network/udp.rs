extern crate alloc;
use alloc::vec::Vec;
use alloc::vec;

use crate::network::device::NetworkDevice;
use crate::network::arp::ArpCache;
use crate::network::device::Result as NetResult;
use crate::network::device::NetError;
use crate::network::checksums;
use crate::network::ipv4;
use crate::network::ethernet::{build_eth_frame, ETHERTYPE_IPV4};

/// Simple UDP socket implementation (small, synchronous API).
///
/// Note: this is a minimal, non-blocking API. `send_to` will return
/// `Err(NetError::WouldBlock)` if the destination MAC is unknown (ARP miss).
pub struct UdpSocket {
    bound_port: u16,
    // recv_queue stores tuples (payload, src_ip, src_port) in FIFO order
    recv_queue: Vec<(Vec<u8>, [u8;4], u16)>,
}

impl UdpSocket {
    pub fn bind(port: u16) -> Self {
        UdpSocket { bound_port: port, recv_queue: Vec::new() }
    }

    /// Build a UDP packet (header + payload) into `out` and return length.
    fn build_udp_packet(src_port: u16, dst_port: u16, payload: &[u8], src_ip: [u8;4], dst_ip: [u8;4], out: &mut [u8]) -> Option<usize> {
        let udp_len = 8usize + payload.len();
        if out.len() < udp_len { return None; }
        // src port
        out[0..2].copy_from_slice(&src_port.to_be_bytes());
        out[2..4].copy_from_slice(&dst_port.to_be_bytes());
        out[4..6].copy_from_slice(&(udp_len as u16).to_be_bytes());
        // checksum initially zero
        out[6] = 0; out[7] = 0;
        out[8..8+payload.len()].copy_from_slice(payload);

        // compute checksum over pseudo-header + udp
        let ck = checksums::udp_checksum(src_ip, dst_ip, &out[..udp_len]);
        let ck_be = ck.to_be_bytes();
        out[6] = ck_be[0]; out[7] = ck_be[1];
        Some(udp_len)
    }

    /// Parse an incoming UDP packet (header+payload). Returns (src_port,dst_port,payload_slice)
    pub fn parse_udp_packet(buf: &[u8]) -> Option<(u16,u16,&[u8])> {
        if buf.len() < 8 { return None; }
        let src_port = u16::from_be_bytes([buf[0], buf[1]]);
        let dst_port = u16::from_be_bytes([buf[2], buf[3]]);
        let udp_len = u16::from_be_bytes([buf[4], buf[5]]) as usize;
        if udp_len < 8 { return None; }
        if buf.len() != udp_len { return None; }
        let payload = &buf[8..];
        Some((src_port, dst_port, payload))
    }

    /// Send data to destination IP:port using the provided device and ARP cache.
    ///
    /// If the ARP cache has no entry for `dst_ip` this returns `Err(NetError::WouldBlock)`
    /// (caller can arrange ARP resolution separately).
    pub fn send_to(&mut self, src_ip: [u8;4], dst_ip: [u8;4], dst_port: u16, data: &[u8], device: &mut dyn NetworkDevice, arp: &mut ArpCache) -> NetResult<()> {
        // build UDP payload
        let mut udp_buf = vec![0u8; 8 + data.len()];
        let _ = Self::build_udp_packet(self.bound_port, dst_port, data, src_ip, dst_ip, &mut udp_buf).ok_or(NetError::BufferTooSmall)?;

        // encapsulate in IPv4
        let mut ip_out = vec![0u8; 20 + udp_buf.len()];
        let ip_len = ipv4::build_ipv4_packet(src_ip, dst_ip, 17u8, &udp_buf, &mut ip_out).ok_or(NetError::BufferTooSmall)?;

        // resolve dst MAC via ARP cache
        if let Some(dst_mac) = arp.lookup(dst_ip) {
            let src_mac = device.mac_addr();
            let mut frame = vec![0u8; 14 + ip_len];
            if let Some(frame_len) = build_eth_frame(dst_mac, src_mac, ETHERTYPE_IPV4, &ip_out[..ip_len], &mut frame) {
                device.transmit(&frame[..frame_len])?;
                return Ok(());
            } else {
                return Err(NetError::BufferTooSmall);
            }
        }

        // ARP miss: upper layer should trigger ARP request; indicate WouldBlock
        Err(NetError::WouldBlock)
    }

    /// Receive next packet if available. Returns (payload, (src_ip, src_port))
    pub fn recv_from(&mut self) -> Option<(Vec<u8>, ([u8;4], u16))> {
        if self.recv_queue.is_empty() { return None; }
        // FIFO: remove first element
        let (buf, src_ip, src_port) = self.recv_queue.remove(0);
        Some((buf, (src_ip, src_port)))
    }

    /// Return the port this socket is bound to
    pub fn bound_port(&self) -> u16 { self.bound_port }

    /// Enqueue an incoming UDP payload (called by demux when a packet arrives).
    pub fn enqueue_incoming(&mut self, src_ip: [u8;4], src_port: u16, payload: &[u8]) {
        self.recv_queue.push((payload.to_vec(), src_ip, src_port));
    }
}

