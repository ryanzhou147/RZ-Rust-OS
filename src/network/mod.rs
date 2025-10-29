pub mod device;
pub mod e1000;
pub mod buf;
pub mod ethernet;
pub mod arp;
pub mod ipv4;
pub mod icmp;
pub mod udp;
pub mod sockets;
pub mod checksums;

/// Initialize the network stack with a device. (skeleton)
pub fn init(_device: &'static mut dyn device::NetworkDevice) {
    // TODO: store device reference in static and initialize modules
}

/// Poll function to run periodic background tasks (skeleton)
pub fn poll() {
    // TODO: process RX/TX, timers, ARP timeouts
}

#[cfg(test)]
mod tests {
    use crate::network::device::{NetworkDevice, MacAddr, NetError};
    use crate::network::e1000::E1000;

    struct Stub;
    impl NetworkDevice for Stub {
        fn transmit(&mut self, _frame: &[u8]) -> core::result::Result<(), NetError> { Ok(()) }
        fn receive(&mut self, _buf: &mut [u8]) -> core::result::Result<usize, NetError> { Err(NetError::WouldBlock) }
        fn mac_addr(&self) -> MacAddr { [0u8;6] }
        fn mtu(&self) -> usize { 1500 }
        fn handle_interrupt(&mut self) {}
    }

    #[test]
    fn stack_init_compile() {
        let mut dev = Stub;
        let dev_ref: &'static mut dyn NetworkDevice = &mut dev;
        crate::network::init(dev_ref);
        crate::network::poll();
    }

    #[test]
    fn e1000_construct() {
        let mut d = E1000::new(0);
        let _ = d.init();
    }
}
