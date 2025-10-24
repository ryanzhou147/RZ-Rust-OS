/// Minimal BlockDevice trait used by the FAT modules.
pub trait BlockDevice {
    /// Read exactly 512 bytes from LBA into `buf`.
    fn read_sector(&self, lba: u64, buf: &mut [u8]);
    /// Write exactly 512 bytes from `data` into LBA.
    fn write_sector(&mut self, lba: u64, data: &[u8]);
    /// Number of 512-byte sectors on this device.
    fn sector_count(&self) -> u64;
}
