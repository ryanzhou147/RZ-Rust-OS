use crate::fs::block_device::BlockDevice;

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
}
