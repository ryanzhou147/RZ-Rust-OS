// Constants for the FAT12 filesystem used in this project.
// Keep these in a small module so tests and parsers can import them.

pub const BYTES_PER_SECTOR: u16 = 512;
pub const SECTORS_PER_CLUSTER_DEFAULT: u8 = 1;
pub const NUM_FATS: u8 = 1; // single FAT copy for simplicity
pub const FAT12_MAX_ROOT_DIR_ENTRIES: u16 = 224; // common floppy default

// FAT12 detection thresholds
pub const FAT12_MAX_CLUSTERS: u32 = 4084; // < 4085 means FAT12

// Boot sector signature offset
pub const BOOT_SIG_OFFSET: usize = 510;
pub const BOOT_SIG_LEAD: u8 = 0x55;
pub const BOOT_SIG_TRAIL: u8 = 0xAA;
