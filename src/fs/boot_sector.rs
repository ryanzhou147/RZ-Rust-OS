use crate::fs::fat_constants::{BOOT_SIG_LEAD, BOOT_SIG_TRAIL};
use core::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum FatError {
    InvalidLength,
    InvalidSignature,
}

impl fmt::Display for FatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FatError::InvalidLength => write!(f, "invalid boot sector length"),
            FatError::InvalidSignature => write!(f, "invalid boot signature"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub max_root_dir_entries: u16,
    pub total_sectors: u16,
    pub sectors_per_fat: u16,
    pub fat_start_lba: u32,
    pub root_dir_start_lba: u32,
    pub data_start_lba: u32,
}

impl BootSector {
    pub fn parse(buf: &[u8]) -> Result<Self, FatError> {
        if buf.len() < 512 { return Err(FatError::InvalidLength); }
        if buf[510] != BOOT_SIG_LEAD || buf[511] != BOOT_SIG_TRAIL { return Err(FatError::InvalidSignature); }

        let bytes_per_sector = u16::from_le_bytes([buf[11], buf[12]]);
        let sectors_per_cluster = buf[13];
        let reserved_sectors = u16::from_le_bytes([buf[14], buf[15]]);
        let num_fats = buf[16];
        let max_root_dir_entries = u16::from_le_bytes([buf[17], buf[18]]);
        let total_sectors = u16::from_le_bytes([buf[19], buf[20]]);
        let sectors_per_fat = u16::from_le_bytes([buf[22], buf[23]]);

        let fat_start_lba = reserved_sectors as u32;
        let root_dir_start_lba = fat_start_lba + (num_fats as u32 * sectors_per_fat as u32);
        let root_dir_sectors = ((max_root_dir_entries as u32 * 32) + (bytes_per_sector as u32 - 1)) / bytes_per_sector as u32;
        let data_start_lba = root_dir_start_lba + root_dir_sectors;

        Ok(BootSector {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            max_root_dir_entries,
            total_sectors,
            sectors_per_fat,
            fat_start_lba,
            root_dir_start_lba,
            data_start_lba,
        })
    }

    pub fn serialize(&self, buf: &mut [u8]) -> Result<(), FatError> {
        if buf.len() < 512 { return Err(FatError::InvalidLength); }
        buf[11..13].copy_from_slice(&self.bytes_per_sector.to_le_bytes());
        buf[13] = self.sectors_per_cluster;
        buf[14..16].copy_from_slice(&self.reserved_sectors.to_le_bytes());
        buf[16] = self.num_fats;
        buf[17..19].copy_from_slice(&self.max_root_dir_entries.to_le_bytes());
        buf[19..21].copy_from_slice(&self.total_sectors.to_le_bytes());
        buf[22..24].copy_from_slice(&self.sectors_per_fat.to_le_bytes());
        // boot sig
        buf[510] = BOOT_SIG_LEAD;
        buf[511] = BOOT_SIG_TRAIL;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::fat_constants::FAT12_MAX_ROOT_DIR_ENTRIES;

    #[test_case]
    fn parse_1440_boot_sector() {
        let mut buf = [0u8; 512];
        // minimal BPB for 1.44MB
        buf[11..13].copy_from_slice(&512u16.to_le_bytes());
        buf[13] = 1; // sectors per cluster
        buf[14..16].copy_from_slice(&1u16.to_le_bytes()); // reserved
        buf[16] = 1; // num_fats
        buf[17..19].copy_from_slice(&FAT12_MAX_ROOT_DIR_ENTRIES.to_le_bytes());
        buf[19..21].copy_from_slice(&2880u16.to_le_bytes());
        buf[22..24].copy_from_slice(&9u16.to_le_bytes());
        buf[510] = BOOT_SIG_LEAD;
        buf[511] = BOOT_SIG_TRAIL;

        let bs = BootSector::parse(&buf).expect("parse failed");
        assert_eq!(bs.bytes_per_sector, 512);
        assert_eq!(bs.sectors_per_cluster, 1);
        assert_eq!(bs.reserved_sectors, 1);
        assert_eq!(bs.num_fats, 1);
        assert_eq!(bs.max_root_dir_entries, FAT12_MAX_ROOT_DIR_ENTRIES);
        assert_eq!(bs.total_sectors, 2880);
        assert_eq!(bs.sectors_per_fat, 9);
        assert_eq!(bs.fat_start_lba, 1);
        assert_eq!(bs.root_dir_start_lba, 1 + 1 * 9);
        // compute expected root_dir_sectors
        let root_dir_sectors = ((FAT12_MAX_ROOT_DIR_ENTRIES as u32 * 32) + (512 - 1)) / 512;
        assert_eq!(bs.data_start_lba, bs.root_dir_start_lba + root_dir_sectors);
    }

    #[test_case]
    fn serialize_and_parse_roundtrip() {
        let bs = BootSector {
            bytes_per_sector: 512,
            sectors_per_cluster: 1,
            reserved_sectors: 1,
            num_fats: 1,
            max_root_dir_entries: FAT12_MAX_ROOT_DIR_ENTRIES,
            total_sectors: 2880,
            sectors_per_fat: 9,
            fat_start_lba: 1,
            root_dir_start_lba: 10,
            data_start_lba: 20,
        };
        let mut buf = [0u8; 512];
        bs.serialize(&mut buf).expect("serialize failed");
        let bs2 = BootSector::parse(&buf).expect("parse failed");
        assert_eq!(bs2.bytes_per_sector, 512);
        assert_eq!(bs2.total_sectors, 2880);
    }
}
