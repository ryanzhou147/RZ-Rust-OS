extern crate alloc;
use alloc::vec::Vec;

use crate::network::device::NetworkDevice;
use crate::network::arp::ArpCache;
use crate::network::device::Result as NetResult;
use crate::network::device::NetError;

/// Simple UDP socket skeleton
pub struct UdpSocket {
    bound_port: u16,
    recv_queue: Vec<Vec<u8>>,
}

impl UdpSocket {
    pub fn bind(port: u16) -> Self {
        UdpSocket { bound_port: port, recv_queue: Vec::new() }
    }

    /// Send data to destination IP:port using the provided device and ARP cache.
    pub fn send_to(&mut self, _dst_ip: [u8;4], _dst_port: u16, _data: &[u8], _device: &mut dyn NetworkDevice, _arp: &mut ArpCache) -> NetResult<()> {
        // TODO: build UDP header, encapsulate in IPv4/Ethernet, resolve ARP and transmit
        Err(NetError::WouldBlock)
    }

    /// Receive next packet if available
    pub fn recv_from(&mut self) -> Option<(Vec<u8>, ([u8;4], u16))> {
        // TODO: pop from recv_queue
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn udp_bind() {
        let s = UdpSocket::bind(1234);
        assert_eq!(s.bound_port, 1234);
    }
}
