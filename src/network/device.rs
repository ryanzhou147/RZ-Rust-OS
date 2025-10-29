#![no_std]

use core::fmt;

pub type MacAddr = [u8; 6];
pub type Result<T> = core::result::Result<T, NetError>;

#[derive(Debug, PartialEq, Eq)]
pub enum NetError {
    WouldBlock,
    DeviceFailure,
    BufferTooSmall,
    Unsupported,
}

/// Device ↔ stack interface.
pub trait NetworkDevice {
    /// Transmit a frame (copy-based). Returns Ok(()) on success.
    fn transmit(&mut self, frame: &[u8]) -> Result<()>;

    /// Try to receive a frame into the provided buffer.
    /// Returns number of bytes written or WouldBlock.
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Return device MAC address
    fn mac_addr(&self) -> MacAddr;

    /// MTU (in bytes)
    fn mtu(&self) -> usize;

    /// Called by interrupt handler to notify driver of new RX/TX events.
    /// The stack will call this when it wants the driver to poll state.
    fn handle_interrupt(&mut self);
}

// Tests: compile-only shape checks
#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;
    impl NetworkDevice for Dummy {
        fn transmit(&mut self, _frame: &[u8]) -> Result<()> { Ok(()) }
        fn receive(&mut self, _buf: &mut [u8]) -> Result<usize> { Err(NetError::WouldBlock) }
        fn mac_addr(&self) -> MacAddr { [0u8;6] }
        fn mtu(&self) -> usize { 1500 }
        fn handle_interrupt(&mut self) {}
    }

    #[test]
    fn trait_shape_compile() {
        let mut d = Dummy;
        let mut buf = [0u8; 64];
        let _ = d.transmit(&buf);
        let _ = d.receive(&mut buf);
        assert_eq!(d.mtu(), 1500);
    }
}
