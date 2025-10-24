use crate::fs::block_device::BlockDevice;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: [u8; 8],
    pub ext: [u8; 3],
    pub attr: u8,
    pub reserved: [u8; 10],
    pub start_cluster: u16,
    pub file_size: u32,
}

impl DirectoryEntry {
    pub fn empty() -> Self {
        DirectoryEntry {
            name: [0u8; 8],
            ext: [0u8; 3],
            attr: 0,
            reserved: [0u8; 10],
            start_cluster: 0,
            file_size: 0,
        }
    }
}

pub struct Directory<'a, D: BlockDevice> {
    device: &'a mut D,
    start_lba: u64,
    num_entries: u16,
}

impl<'a, D: BlockDevice> Directory<'a, D> {
    pub fn new(device: &'a mut D, start_lba: u64, num_entries: u16) -> Self {
        Directory { device, start_lba, num_entries }
    }

    fn read_entry_raw(&mut self, idx: usize, out: &mut [u8; 32]) {
        let entries_per_sector = 512 / 32;
        let sector_idx = idx / entries_per_sector;
        let sector_off = idx % entries_per_sector;
        let mut sector = [0u8; 512];
        self.device.read_sector(self.start_lba + sector_idx as u64, &mut sector);
        let start = sector_off * 32;
        out.copy_from_slice(&sector[start..start+32]);
    }

    fn write_entry_raw(&mut self, idx: usize, data: &[u8; 32]) {
        let entries_per_sector = 512 / 32;
        let sector_idx = idx / entries_per_sector;
        let sector_off = idx % entries_per_sector;
        let mut sector = [0u8; 512];
        self.device.read_sector(self.start_lba + sector_idx as u64, &mut sector);
        let start = sector_off * 32;
        sector[start..start+32].copy_from_slice(data);
        self.device.write_sector(self.start_lba + sector_idx as u64, &sector);
    }

    pub fn list(&mut self) -> alloc::vec::Vec<DirectoryEntry> {
        let mut out = alloc::vec::Vec::new();
        for i in 0..self.num_entries as usize {
            let mut raw = [0u8; 32];
            self.read_entry_raw(i, &mut raw);
            if raw[0] == 0x00 { break; }
            if raw[0] == 0xE5 { continue; }
            // parse
            let mut name = [b' '; 8];
            name.copy_from_slice(&raw[0..8]);
            let mut ext = [b' '; 3];
            ext.copy_from_slice(&raw[8..11]);
            let attr = raw[11];
            let start_cluster = u16::from_le_bytes([raw[26], raw[27]]);
            let file_size = u32::from_le_bytes([raw[28], raw[29], raw[30], raw[31]]);
            out.push(DirectoryEntry { name, ext, attr, reserved: [0u8;10], start_cluster, file_size });
        }
        out
    }

    pub fn find(&mut self, name: &str) -> Option<DirectoryEntry> {
        for i in 0..self.num_entries as usize {
            let mut raw = [0u8; 32];
            self.read_entry_raw(i, &mut raw);
            if raw[0] == 0x00 { break; }
            if raw[0] == 0xE5 { continue; }
            let fname = core::str::from_utf8(&raw[0..11]).unwrap_or("");
            let fname_trim = fname.trim_end_matches(' ');
            if fname_trim == name {                
                let mut nameb = [b' '; 8];
                nameb.copy_from_slice(&raw[0..8]);
                let mut ext = [b' ';3]; ext.copy_from_slice(&raw[8..11]);
                let attr = raw[11];
                let start_cluster = u16::from_le_bytes([raw[26], raw[27]]);
                let file_size = u32::from_le_bytes([raw[28], raw[29], raw[30], raw[31]]);
                return Some(DirectoryEntry { name: nameb, ext, attr, reserved: [0u8;10], start_cluster, file_size });
            }
        }
        None
    }

    pub fn create(&mut self, name: &str, start_cluster: u16, size: u32) {
        // find first free entry
        for i in 0..self.num_entries as usize {
            let mut raw = [0u8; 32];
            self.read_entry_raw(i, &mut raw);
            if raw[0] == 0x00 || raw[0] == 0xE5 {
                // write entry
                let mut entry = [0u8; 32];
                let mut name_buf = [b' '; 11];
                for (j, b) in name.as_bytes().iter().take(11).enumerate() { name_buf[j] = *b; }
                entry[0..11].copy_from_slice(&name_buf);
                entry[11] = 0; // attr
                entry[26..28].copy_from_slice(&start_cluster.to_le_bytes());
                entry[28..32].copy_from_slice(&size.to_le_bytes());
                self.write_entry_raw(i, &entry);
                return;
            }
        }
    }

    pub fn delete(&mut self, name: &str) {
        for i in 0..self.num_entries as usize {
            let mut raw = [0u8; 32];
            self.read_entry_raw(i, &mut raw);
            if raw[0] == 0x00 { break; }
            if raw[0] == 0xE5 { continue; }
            let fname = core::str::from_utf8(&raw[0..11]).unwrap_or("");
            let fname_trim = fname.trim_end_matches(' ');
            if fname_trim == name {
                raw[0] = 0xE5; // mark deleted
                self.write_entry_raw(i, &raw);
                return;
            }
        }
    }

    pub fn serialize(&mut self, buf: &mut [u8]) {
        // writes current directory region into buf
        for i in 0..self.num_entries as usize {
            let mut raw = [0u8; 32];
            self.read_entry_raw(i, &mut raw);
            buf[i*32..i*32+32].copy_from_slice(&raw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // using MockDeviceFixed in this test

    #[test_case]
    fn dir_create_serialize() {
        static mut BUF: [u8; 512 * 2] = [0u8; 512 * 2];
        unsafe {
            let buf = &mut BUF[..];
            let mut dev = crate::fs::mock_device::MockDevice::new(buf);
            let mut dir = Directory::new(&mut dev, 0, 32);
            dir.create("FOO     TXT", 2, 12);
            dir.create("BAR     TXT", 3, 7);
            let list = dir.list();
            assert_eq!(list.len(), 2);
            assert_eq!(list[0].start_cluster, 2);
            assert_eq!(list[0].file_size, 12);
            assert_eq!(list[1].start_cluster, 3);
            assert_eq!(list[1].file_size, 7);
            // serialize into a buffer and verify first entry bytes
            let mut out = [0u8; 32*32];
            dir.serialize(&mut out);
            assert_eq!(&out[0..11], b"FOO     TXT");
            assert_eq!(&out[32..43], b"BAR     TXT");
        }
    }
}
