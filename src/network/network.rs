use crate::network::ethernet::{parse_eth_header, build_eth_frame, ETHERTYPE_IPV4};
use crate::network::ipv4::{parse_ipv4_header, build_ipv4_packet};
use crate::network::checksums;
use crate::network::udp::UdpSocket;
use crate::network::arp::ArpCache;
use crate::network::device::Result as NetResult;

extern crate alloc;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::network::device::{NetworkDevice, NetError};

/// Opaque pointer to the active device. We keep a raw pointer to avoid
/// ownership/borrow issues across the kernel; callers must pass a leaked
/// `'static mut` device (Box::leak) as usual in this project.
/// Initialize the network stack with a device. For now this is a no-op; callers
/// can use `poll_device()` to drive a concrete device directly.
/// Global boxed device storage. We store a boxed `dyn NetworkDevice + Send` inside
/// a spin `Mutex` guarded by a `OnceCell`. This keeps the storage safe to access
/// from different execution contexts while avoiding raw fat-pointer global issues.
static DEVICE: OnceCell<Mutex<Box<dyn NetworkDevice + Send>>> = OnceCell::uninit();

/// Simple UDP socket registry (vector protected by spin mutex). This is a
/// very small registry used by `poll_device` to find a socket bound to a
/// destination port and enqueue incoming datagrams.
static UDP_SOCKETS: OnceCell<Mutex<Vec<crate::network::udp::UdpSocket>>> = OnceCell::uninit();

/// Register a `UdpSocket` into the global socket registry. Returns `true` on
/// success. Registration will fail (return false) if another socket is
/// already bound to the same port.
pub fn register_udp_socket(sock: crate::network::udp::UdpSocket) -> bool {
    let port = sock.bound_port();
    // ensure registry is initialized
    let _ = UDP_SOCKETS.try_init_once(|| Mutex::new(Vec::new()));
    if let Ok(m) = UDP_SOCKETS.try_get() {
        let mut vec = m.lock();
        // prevent duplicate bindings
        if vec.iter().any(|s| s.bound_port() == port) {
            return false;
        }
        vec.push(sock);
        return true;
    }
    false
}

/// Unregister a socket bound to `port`. Returns `true` if a socket was
/// removed, `false` if none was bound.
pub fn unregister_udp_socket(port: u16) -> bool {
    if let Ok(m) = UDP_SOCKETS.try_get() {
        let mut vec = m.lock();
        if let Some(idx) = vec.iter().position(|s| s.bound_port() == port) {
            vec.remove(idx);
            return true;
        }
    }
    false
}

/// Convenience handle returned to callers after binding. The handle holds the
/// bound port and performs operations by locking the global socket registry.
pub struct UdpHandle {
    pub port: u16,
}

impl UdpHandle {
    /// Send data from `src_ip` to `dst_ip:dst_port` using the underlying
    /// socket registered for `self.port`. Returns NetResult as the socket
    /// implementation.
    pub fn send_to(&self, src_ip: [u8;4], dst_ip: [u8;4], dst_port: u16, data: &[u8], device: &mut dyn NetworkDevice, arp: &mut ArpCache) -> NetResult<()> {
        if let Ok(m) = UDP_SOCKETS.try_get() {
            let mut vec = m.lock();
            if let Some(s) = vec.iter_mut().find(|s| s.bound_port() == self.port) {
                return s.send_to(src_ip, dst_ip, dst_port, data, device, arp);
            }
        }
        Err(NetError::WouldBlock)
    }

    /// Try to receive a packet from the bound socket. Returns the payload and
    /// source (ip, port) if available.
    pub fn recv_from(&self) -> Option<(Vec<u8>, ([u8;4], u16))> {
        if let Ok(m) = UDP_SOCKETS.try_get() {
            let mut vec = m.lock();
            if let Some(s) = vec.iter_mut().find(|s| s.bound_port() == self.port) {
                return s.recv_from();
            }
        }
        None
    }
}

/// Bind and register a UDP socket in one step. Returns a `UdpHandle` on
/// success or `None` if the port is already bound.
pub fn bind_udp_socket(port: u16) -> Option<UdpHandle> {
    let sock = UdpSocket::bind(port);
    if register_udp_socket(sock) {
        Some(UdpHandle { port })
    } else {
        None
    }
}

/// Initialize the network stack with a concrete device. The device is boxed and
/// stored globally for later polling. The device type must implement `Send` so
/// the boxed trait object is safe to put behind a `Mutex` in a static.
pub fn init<D>(device: D)
where
    D: NetworkDevice + Send + 'static,
{
    let _ = DEVICE.try_init_once(|| Mutex::new(Box::new(device) as Box<dyn NetworkDevice + Send>));
}

/// Poll and demux frames for a provided device. This function performs the
/// receive loop and dispatches IPv4/ICMP processing. It intentionally takes a
/// `&mut dyn NetworkDevice` so callers can manage device ownership.
pub fn poll_device(dev: &mut dyn NetworkDevice) {
    let mtu = dev.mtu();
    let mut buf = [0u8; 2048];

    loop {
        match dev.receive(&mut buf[..mtu]) {
            Ok(len) => {
                if len == 0 { continue; }
                // parse ethernet
                if let Some((eth_hdr, payload)) = parse_eth_header(&buf[..len]) {
                    if eth_hdr.ethertype == ETHERTYPE_IPV4 {
                        if let Some((ip_hdr, ip_payload)) = parse_ipv4_header(payload) {
                            match ip_hdr.proto {
                                1 => {
                                    // ICMP: call handler
                                    if let Some(reply_payload) = crate::network::icmp::handle_icmp(&ip_hdr, ip_payload) {
                                        // build IPv4 packet (swap src/dst)
                                        let mut ip_out = [0u8; 1600];
                                        if let Some(ip_len) = build_ipv4_packet(ip_hdr.dst, ip_hdr.src, 1u8, &reply_payload, &mut ip_out) {
                                            // build Ethernet frame (dst=eth_hdr.src, src=device mac)
                                            let mut frame = [0u8; 1600];
                                            let src_mac = dev.mac_addr();
                                            if let Some(frame_len) = build_eth_frame(eth_hdr.src, src_mac, ETHERTYPE_IPV4, &ip_out[..ip_len], &mut frame) {
                                                let _ = dev.transmit(&frame[..frame_len]);
                                            }
                                        }
                                    }
                                }
                                17 => {
                                    // UDP: verify checksum then dispatch to matching socket
                                    if checksums::verify_udp_checksum(ip_hdr.src, ip_hdr.dst, ip_payload) {
                                        if let Some((src_port, dst_port, payload)) = crate::network::udp::UdpSocket::parse_udp_packet(ip_payload) {
                                            // try to find a registered socket and enqueue
                                            if let Ok(mutex) = UDP_SOCKETS.try_get() {
                                                let mut vec = mutex.lock();
                                                for s in vec.iter_mut() {
                                                    if s.bound_port() == dst_port {
                                                        s.enqueue_incoming(ip_hdr.src, src_port, payload);
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    // TODO: handle TCP/other protocols
                                }
                            }
                        }
                    }
                }
            }
            Err(NetError::WouldBlock) => break,
            Err(_) => break,
        }
    }
}

/// Poll using a stored device if one was configured via `init()` (currently no-op)
/// Poll the globally stored device (if any). This calls `poll_device` with the
/// boxed device held in the global `DEVICE` OnceCell.
pub fn poll() {
    if let Ok(mutex) = DEVICE.try_get() {
        let mut guard = mutex.lock();
        // guard: &mut Box<dyn NetworkDevice + Send>
        let dev: &mut dyn NetworkDevice = guard.as_mut();
        poll_device(dev);
    }
}

