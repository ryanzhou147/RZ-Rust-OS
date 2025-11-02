use crate::network::ethernet::{parse_eth_header, build_eth_frame, ETHERTYPE_IPV4};
use crate::network::ipv4::{parse_ipv4_header, build_ipv4_packet};

extern crate alloc;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use alloc::boxed::Box;
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
                                _ => {
                                    // TODO: handle UDP/TCP/etc
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

