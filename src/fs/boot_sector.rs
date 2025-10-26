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

// Unit tests for BootSector moved to tests/fs_integration.rs
