use crate::fs::block_device::BlockDevice;

/// A borrowed, slice-backed mock device (keeps compatibility with existing tests).
pub struct MockDevice<'a> {
    pub buf: &'a mut [u8], // must be multiple of 512
}

impl<'a> MockDevice<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self { MockDevice { buf } }
    pub fn sector_count(&self) -> u64 { (self.buf.len() / 512) as u64 }
}

impl<'a> BlockDevice for MockDevice<'a> {
    fn read_sector(&self, lba: u64, buf: &mut [u8]) {
        let start = lba as usize * 512;
        buf.copy_from_slice(&self.buf[start..start+512]);
    }
    fn write_sector(&mut self, lba: u64, data: &[u8]) {
        let start = lba as usize * 512;
        self.buf[start..start+512].copy_from_slice(data);
    }
    fn sector_count(&self) -> u64 { (self.buf.len() / 512) as u64 }
}

/// Owned, fixed-size mock device suitable for unit tests and host-side tooling.
pub struct MockDeviceFixed {
    pub storage: [u8; 1_474_560], // 1.44MB floppy image size
}

impl MockDeviceFixed {
    /// Const constructor so it can be used in `static` initializer in tests.
    pub const fn new() -> Self {
        MockDeviceFixed { storage: [0u8; 1_474_560] }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] { &mut self.storage }
}

impl BlockDevice for MockDeviceFixed {
    fn read_sector(&self, lba: u64, buf: &mut [u8]) {
        let start = lba as usize * 512;
        buf.copy_from_slice(&self.storage[start..start+512]);
    }

    fn write_sector(&mut self, lba: u64, data: &[u8]) {
        let start = lba as usize * 512;
        self.storage[start..start+512].copy_from_slice(data);
    }

    fn sector_count(&self) -> u64 { (self.storage.len() / 512) as u64 }
}
