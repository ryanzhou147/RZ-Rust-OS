use crate::fs::block_device::BlockDevice;

pub struct FatTable<'a, D: BlockDevice> {
    device: &'a mut D,
    start_lba: u64,
    sectors_per_fat: u16,
    // simple cache: one sector buffer
    cache_sector: u32,
    cache: [u8; 512],
    cache_dirty: bool,
}

impl<'a, D: BlockDevice> FatTable<'a, D> {
    pub fn new(device: &'a mut D, start_lba: u64, sectors_per_fat: u16) -> Self {
        FatTable {
            device,
            start_lba,
            sectors_per_fat,
            cache_sector: u32::MAX,
            cache: [0u8; 512],
            cache_dirty: false,
        }
    }

    fn load_sector(&mut self, sector_idx: u32) {
        if self.cache_sector == sector_idx { return; }
        if self.cache_dirty { self.flush().unwrap_or(()); }
        self.device.read_sector(self.start_lba + sector_idx as u64, &mut self.cache);
        self.cache_sector = sector_idx;
        self.cache_dirty = false;
    }

    pub fn flush(&mut self) -> Result<(), ()> {
        if !self.cache_dirty { return Ok(()); }
        self.device.write_sector(self.start_lba + self.cache_sector as u64, &self.cache);
        self.cache_dirty = false;
        Ok(())
    }

    /// Helper to write a data-sector using the underlying device borrowed by the FatTable.
    pub fn write_data_sector(&mut self, lba: u64, data: &[u8]) {
        self.device.write_sector(lba, data);
    }

    /// Helper to read a data-sector using the underlying device borrowed by the FatTable.
    pub fn read_data_sector(&mut self, lba: u64, buf: &mut [u8]) {
        self.device.read_sector(lba, buf);
    }

    /// Read a FAT12 entry (12-bit) for cluster `n`.
    pub fn read_entry(&mut self, cluster: u16) -> u16 {
        // index into FAT bytes
        let idx = (cluster as usize * 3) / 2;
        let sector_idx = (idx / 512) as u32;
        let offset = idx % 512;
        self.load_sector(sector_idx);
        // need potentially two bytes across sector boundary
        let b0 = self.cache[offset] as u16;
        let b1 = if offset + 1 < 512 { self.cache[offset+1] as u16 } else {
            // read next sector
            self.load_sector(sector_idx + 1);
            self.cache[0] as u16
        };
        let word = (b1 << 8) | b0;
        if cluster % 2 == 0 {
            word & 0x0FFF
        } else {
            (word >> 4) & 0x0FFF
        }
    }

    /// Write a FAT12 entry.
    pub fn write_entry(&mut self, cluster: u16, value: u16) {
        let idx = (cluster as usize * 3) / 2;
        let sector_idx = (idx / 512) as u32;
        let offset = idx % 512;
        self.load_sector(sector_idx);
        // get next byte possibly in next sector
        let mut next_byte = if offset + 1 < 512 { self.cache[offset+1] } else {
            // read next sector into temp
            let mut tmp = [0u8; 512];
            self.device.read_sector(self.start_lba + (sector_idx + 1) as u64, &mut tmp);
            tmp[0]
        } as u16;
        let cur = self.cache[offset] as u16;
        let mut word = (next_byte << 8) | cur;
        if cluster % 2 == 0 {
            // clear low 12 bits and set
            word = (word & 0xF000) | (value & 0x0FFF);
        } else {
            // clear high 12 bits and set
            word = (word & 0x000F) | ((value & 0x0FFF) << 4);
        }
        // write back
        let new_b0 = (word & 0xFF) as u8;
        let new_b1 = ((word >> 8) & 0xFF) as u8;
        self.cache[offset] = new_b0;
        self.cache_dirty = true;
        if offset + 1 < 512 {
            self.cache[offset+1] = new_b1;
        } else {
            // write next sector's first byte
            let mut tmp = [0u8; 512];
            self.device.read_sector(self.start_lba + (sector_idx + 1) as u64, &mut tmp);
            tmp[0] = new_b1;
            self.device.write_sector(self.start_lba + (sector_idx + 1) as u64, &tmp);
        }
    }

    /// Find a free cluster (value 0) and allocate it (set to 0xFFF end)
    pub fn alloc_cluster(&mut self) -> Option<u16> {
        // naive scan clusters starting at 2
        let max_bytes = (self.sectors_per_fat as usize) * 512;
        let max_clusters = (max_bytes * 2) / 3; // approx
        for n in 2..(max_clusters as u16) {
            if self.read_entry(n) == 0 {
                self.write_entry(n, 0xFFF);
                return Some(n);
            }
        }
        None
    }

    /// Free a cluster chain starting at `cluster`.
    pub fn free_cluster(&mut self, cluster: u16) {
        let mut cur = cluster;
        loop {
            let next = self.read_entry(cur);
            self.write_entry(cur, 0);
            if next >= 0xFF8 { break; }
            cur = next;
        }
    }

    /// Follow chain starting at `start` until EOF and return vector of clusters.
    pub fn get_chain(&mut self, start: u16) -> alloc::vec::Vec<u16> {
        let mut out = alloc::vec::Vec::new();
        let mut cur = start;
        loop {
            out.push(cur);
            let next = self.read_entry(cur);
            if next >= 0xFF8 { break; }
            cur = next;
        }
        out
    }

    /// Non-alloc version: fill provided slice with the cluster chain and return the length.
    pub fn get_chain_nonalloc(&mut self, start: u16, out: &mut [u16]) -> usize {
        let mut idx = 0usize;
        let mut cur = start;
        loop {
            if idx >= out.len() { break; }
            out[idx] = cur;
            idx += 1;
            let next = self.read_entry(cur);
            if next >= 0xFF8 { break; }
            cur = next;
        }
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // using MockDeviceFixed in this test

    #[test_case]
    fn fat12_read_write_simple() {
        static mut BUF: [u8; 512 * 9] = [0u8; 512 * 9]; // 9 sectors FAT
        unsafe {
            let buf = &mut BUF[..];
            let mut dev = crate::fs::mock_device::MockDevice::new(buf);
            let mut fat = FatTable::new(&mut dev, 0, 9);
            // write cluster 2->3, 3->EOF
            fat.write_entry(2, 3);
            fat.write_entry(3, 0xFFF);
            let v2 = fat.read_entry(2);
            let v3 = fat.read_entry(3);
            assert_eq!(v2, 3);
            assert_eq!(v3, 0xFFF);
            let mut out = [0u16; 16];
            let len = fat.get_chain_nonalloc(2, &mut out);
            assert_eq!(len, 2);
            assert_eq!(out[0], 2);
            assert_eq!(out[1], 3);
        }
    }
}
