FAT12-like format (for RZ-Rust-OS)

This document describes the on-disk layout and constants used by the FAT12-like filesystem implemented in this repository.

High-level assumptions
- Sector size: 512 bytes (logical block size).
- FAT type: FAT12 (12-bit FAT entries).
- Number of FAT copies: 1 (single FAT copy).
- Root directory: flat (no subdirectories supported initially). Root directory occupies a contiguous region immediately following the FAT area.
- Directory entry format: standard 32-byte FAT directory entry. We only use these fields for now:
  - filename (8.3, ASCII, space-padded)
  - attributes (1 byte)
  - starting cluster (u16, little-endian)
  - file size (u32, little-endian)
- Cluster numbering: data clusters start at cluster 2 (cluster values 0 and 1 are reserved by FAT spec).

On-disk layout (sector granularity)
- Sector 0: Boot sector (512 bytes) containing the BIOS Parameter Block (BPB) and the boot signature (0x55 0xAA at offset 510).
- Reserved area: `BPB.reserved_sectors` sectors (BPB usually sets this to 1 for floppies).
- FAT area: `num_fats` copies (here `1`) of the FAT, each occupying `sectors_per_fat` sectors.
- Root directory area: follows the FAT area and occupies `root_dir_sectors` sectors where

    root_dir_sectors = ceil(max_root_dir_entries * 32 / bytes_per_sector)

- Data area: starts immediately after root directory area; contains cluster `2` as the first data cluster.

Key Boot Sector (BPB) fields used
- bytes_per_sector (u16) at offset 11
- sectors_per_cluster (u8) at offset 13
- reserved_sectors (u16) at offset 14
- num_fats (u8) at offset 16
- max_root_dir_entries (u16) at offset 17
- total_sectors_small (u16) at offset 19
- media_descriptor (u8) at offset 21
- sectors_per_fat (u16) at offset 22
- total_sectors_large (u32) at offset 32
- boot signature (0x55AA) at offset 510..512

Derived values (compute after parsing BPB)
- total_sectors = if total_sectors_small != 0 { total_sectors_small } else { total_sectors_large }
- root_dir_sectors = ((max_root_dir_entries as u32 * 32) + (bytes_per_sector - 1)) / bytes_per_sector
- first_data_sector = reserved_sectors as u32 + (num_fats as u32 * sectors_per_fat as u32) + root_dir_sectors
- data_sectors = total_sectors - first_data_sector
- total_clusters = data_sectors / sectors_per_cluster as u32

FAT type detection (simplified)
- If total_clusters < 4085 => FAT12
- If 4085 <= total_clusters < 65525 => FAT16
- Else => FAT32

FAT12 entry layout (brief)
- FAT12 entries are packed 12-bit values. For cluster n:
  - byte_index = (n * 3) / 2
  - If n is even: entry = low 12 bits of little_endian(u16 at byte_index)
  - If n is odd: entry = high 12 bits of little_endian(u16 at byte_index) >> 4

Root directory entries
- Each directory entry is 32 bytes. For this project we only need the following fields (little endian):
  - starting cluster: offset 26..28 (u16)
  - file size: offset 28..32 (u32)
- Special marker bytes:
  - 0x00 : entry and all following entries are unused
  - 0xE5 : deleted entry
  - 0x2E : dot entry (".") or ("..") handling if present

Notes and constraints
- No journaling — write operations can corrupt the FS on power failure.
- Single FAT copy reduces resilience but simplifies writes.
- Flat root directory simplifies lookups; adding subdirectories later requires allocating their clusters in the data area.

References
- Microsoft FAT specification (for details on FAT12 packing and special values).


"Why these choices?"
- 512-byte sectors and FAT12 are ideal for simple floppy-like images (e.g., 1.44MB) and educational purposes.
- Single FAT copy and flat directory keep the implementation small and easy to reason about while still supporting hierarchical data later if needed.