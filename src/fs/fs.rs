use crate::fs::block_device::BlockDevice;
use crate::fs::boot_sector::{BootSector, FatError};
use crate::fs::fat_table::FatTable;
use crate::fs::directory::{Directory, DirectoryEntry};
use crate::println;
use alloc::vec::Vec;
use alloc::vec;
use crate::fs::fat_constants::*;

#[derive(Debug)]
pub enum FsError {
    Boot(FatError),
    FileAlreadyExists,
    FileNotFound,
    NotFound,
    InvalidName,
    NoSpace,
}

impl From<FatError> for FsError {
    fn from(e: FatError) -> Self { FsError::Boot(e) }
}

pub struct FileSystem<'a, D: BlockDevice> {
    device: &'a mut D,
    pub boot_sector: BootSector,
}

impl<'a, D: BlockDevice> FileSystem<'a, D> {
    pub fn mount(device: &'a mut D) -> Result<Self, FsError> {
        let mut buf = [0u8; 512];
        device.read_sector(0, &mut buf);
        let bs = match BootSector::parse(&buf) {
            Ok(b) => b,
            Err(e) => {
                println!("fs::mount: BootSector parse failed: {:?}", e);
                return Err(FsError::Boot(e));
            }
        };
        // build fat and dir using computed LBAs
        Ok(FileSystem { device, boot_sector: bs })
    }

    pub fn list_root(&mut self) -> Vec<DirectoryEntry> {
        let mut dir = Directory::new(
            self.device, 
            self.boot_sector.root_dir_start_lba as u64, 
            self.boot_sector.max_root_dir_entries);
        dir.list()
    }

    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, FsError> {
        // create Directory, find entry
        let entry = {
            let mut dir = Directory::new(self.device, self.boot_sector.root_dir_start_lba as u64, self.boot_sector.max_root_dir_entries);
            match dir.find(name) {
                Some(e) => e,
                None => {
                    // Log missing file for easier debugging in kernel environment
                    return Err(FsError::FileNotFound);
                }
            }
        };
        // follow chain and read clusters using a temporary FatTable
        let chain = {
            let mut fat = FatTable::new(self.device, self.boot_sector.fat_start_lba as u64, self.boot_sector.sectors_per_fat);
            fat.get_chain(entry.start_cluster)
        };
        let mut out: Vec<u8> = Vec::new();
        let bytes_per_sector = self.boot_sector.bytes_per_sector as usize;
        for &cluster in chain.iter() {
            let lba = self.boot_sector.data_start_lba as u64 + ((cluster as u64 - 2) * self.boot_sector.sectors_per_cluster as u64);
            for s in 0..self.boot_sector.sectors_per_cluster as u64 {
                let mut buf = vec![0u8; bytes_per_sector];
                self.device.read_sector(lba + s, &mut buf);
                out.extend_from_slice(&buf);
            }
        }
        out.truncate(entry.file_size as usize);
        Ok(out)
    }

    pub fn write_file(&mut self, name: &str, data: &[u8]) -> Result<(), FsError> {
        // Enforce that files must have a .txt extension.
        // Accept either a human-readable name that ends with ".txt" (case-insensitive),
        // or an already-formatted 8.3 name (11 characters) whose extension bytes 8..11 == "TXT".
        let has_txt_ext = if name.len() == 11 {
            // treat as 8.3 style (e.g. "FOO     TXT")
            match name.get(8..11) {
                Some(ext) => ext.eq_ignore_ascii_case("TXT"),
                None => false,
            }
        } else {
            name.to_ascii_lowercase().ends_with(".txt")
        };
        if !has_txt_ext {
            return Err(FsError::InvalidName);
        }

        // check if file already exists in root directory
        let mut dir_check = Directory::new(
            self.device,
            self.boot_sector.root_dir_start_lba as u64,
            self.boot_sector.max_root_dir_entries,
        );
        if dir_check.find(name).is_some() {
            return Err(FsError::FileAlreadyExists);
        }
        // allocate clusters as needed, write data, update FAT and directory
        let bytes_per_sector = self.boot_sector.bytes_per_sector as usize;
        let sectors_per_cluster = self.boot_sector.sectors_per_cluster as usize;
        let mut remaining = data.len();
        let mut pos = 0usize;
        // allocate clusters using a temporary FatTable and write data
        let mut first_cluster: Option<u16> = None;
        let mut prev_cluster: Option<u16> = None;
        {
            let mut fat = FatTable::new(self.device, self.boot_sector.fat_start_lba as u64, self.boot_sector.sectors_per_fat);
            while remaining > 0 {
                let c = match fat.alloc_cluster() {
                    Some(cc) => cc,
                    None => {
                        return Err(FsError::NoSpace);
                    }
                };
                if first_cluster.is_none() { first_cluster = Some(c); }
                if let Some(pc) = prev_cluster { fat.write_entry(pc, c); }
                prev_cluster = Some(c);
                // write cluster data via fat table helper
                let lba = self.boot_sector.data_start_lba as u64 + ((c as u64 - 2) * self.boot_sector.sectors_per_cluster as u64);
                for s in 0..sectors_per_cluster as u64 {
                    let start = pos;
                    let end = core::cmp::min(pos + bytes_per_sector, data.len());
                    let mut buf = [0u8; 512]; // bytes_per_sector is 512 in our format
                    let slice = &data[start..end];
                    buf[0..slice.len()].copy_from_slice(slice);
                    fat.write_data_sector(lba + s, &buf);
                    pos += slice.len();
                    if pos >= data.len() { break; }
                }
                remaining = data.len() - pos;
            }
            // mark EOF
            if let Some(last) = prev_cluster { fat.write_entry(last, 0xFFF); }
            fat.flush().ok();
        }
        // write directory entry using a temporary Directory
        if let Some(first) = first_cluster {
            let mut dir = Directory::new(self.device, self.boot_sector.root_dir_start_lba as u64, self.boot_sector.max_root_dir_entries);
            dir.create(name, first, data.len() as u32);
            Ok(())
        } else {
            Err(FsError::NoSpace)
        }
    }

    pub fn delete(&mut self, name: &str) -> Result<(), FsError> {
        // find entry
        let entry = {
            let mut dir = Directory::new(self.device, self.boot_sector.root_dir_start_lba as u64, self.boot_sector.max_root_dir_entries);
            match dir.find(name) {
                Some(e) => e,
                None => {
                    return Err(FsError::FileNotFound);
                }
            }
        };
        // free clusters
        {
            let mut fat = FatTable::new(self.device, self.boot_sector.fat_start_lba as u64, self.boot_sector.sectors_per_fat);
            fat.free_cluster(entry.start_cluster);
            fat.flush().ok();
        }
        // delete directory entry
        let mut dir = Directory::new(self.device, self.boot_sector.root_dir_start_lba as u64, self.boot_sector.max_root_dir_entries);
        dir.delete(name);
        Ok(())
    }

    pub fn format(device: &mut D, total_sectors: u16) -> Result<(), FsError> {
        // zero out disk
        let zero = [0u8; 512];
        let sectors = (device.sector_count()) as u64;
        for s in 0..sectors {
            device.write_sector(s, &zero);
        }
        // write boot sector with common FAT12 values
        let bs = BootSector {
            bytes_per_sector: BYTES_PER_SECTOR,
            sectors_per_cluster: SECTORS_PER_CLUSTER_DEFAULT,
            reserved_sectors: 1,
            num_fats: NUM_FATS,
            max_root_dir_entries: FAT12_MAX_ROOT_DIR_ENTRIES,
            total_sectors,
            sectors_per_fat: 9,
            fat_start_lba: 1,
            root_dir_start_lba: 1 + 9,
            data_start_lba: 1 + 9 + (((224u32 * 32) + (512 - 1)) / 512),
        };
        let mut buf = [0u8; 512];
        match bs.serialize(&mut buf) {
            Ok(()) => {}
            Err(e) => {
                println!("fs::format: BootSector serialize failed: {:?}", e);
                return Err(FsError::Boot(e));
            }
        }
        device.write_sector(0, &buf);
        // write empty FAT (zeroed)
        let fat_sectors = bs.sectors_per_fat as u64;
        for i in 0..fat_sectors {
            device.write_sector(bs.fat_start_lba as u64 + i, &zero);
        }
        // write empty root dir (zeroed)
        let root_sectors = (((bs.max_root_dir_entries as u32 * 32) + (512 - 1)) / 512) as u64;
        for i in 0..root_sectors {
            device.write_sector(bs.root_dir_start_lba as u64 + i, &zero);
        }
        Ok(())
    }
}