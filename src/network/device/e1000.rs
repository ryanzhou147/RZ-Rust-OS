use crate::network::device::{NetworkDevice, MacAddr, Result, NetError};

pub struct E1000 {
    mmio_base: usize,
    mac: MacAddr,
    // TODO: descriptor rings, tx/rx indices would go here
}

impl E1000 {
    pub const fn new(mmio_base: usize) -> Self {
        Self { mmio_base, mac: [0u8;6] }
    }

    /// Initialize hardware (placeholder)
    pub fn init(&mut self) -> Result<()> {
        // TODO: PCI/PCIe mapping, reset, configure Rx/Tx rings, read MAC
        Ok(())
    }

    /// Called by IRQ wrapper (external) — schedule work / refill queues
    pub fn interrupt_handler(&mut self) {
        // TODO: acknowledge interrupt and mark RX/TX available
    }
}

impl NetworkDevice for E1000 {
    fn transmit(&mut self, _frame: &[u8]) -> Result<()> {
        // TODO: copy into Tx descriptor, kick the device
        Err(NetError::WouldBlock)
    }

    fn receive(&mut self, _buf: &mut [u8]) -> Result<usize> {
        // TODO: copy from Rx descriptor into provided buffer
        Err(NetError::WouldBlock)
    }

    fn mac_addr(&self) -> MacAddr { self.mac }

    fn mtu(&self) -> usize { 1500 }

    fn handle_interrupt(&mut self) {
        self.interrupt_handler();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_e1000() {
        let mut d = E1000::new(0xfee0_0000);
        assert_eq!(d.mtu(), 1500);
    }
}
