#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rz_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use rz_rust_os::allocator;
use rz_rust_os::memory::{self, BootInfoFrameAllocator};
use x86_64::VirtAddr;

use rz_rust_os::fs::boot_sector::BootSector;
use rz_rust_os::fs::fat_constants::{FAT12_MAX_ROOT_DIR_ENTRIES, BOOT_SIG_LEAD, BOOT_SIG_TRAIL};
use rz_rust_os::fs::mock_device::MockDevice;
use rz_rust_os::fs::fat_table::FatTable;
use rz_rust_os::fs::directory::Directory;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    rz_rust_os::init();
    // Initialize memory and heap so tests can use `alloc` (Vec, Box, etc.).
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization in tests failed");

    test_main();
    loop {}
}

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

#[test_case]
fn fat12_read_write_simple() {
    static mut BUF: [u8; 512 * 9] = [0u8; 512 * 9]; // 9 sectors FAT
    unsafe {
        let buf = &mut BUF[..];
        let mut dev = MockDevice::new(buf);
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

#[test_case]
fn dir_create_serialize() {
    static mut BUF: [u8; 512 * 2] = [0u8; 512 * 2];
    unsafe {
        let buf = &mut BUF[..];
        let mut dev = MockDevice::new(buf);
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
        let mut out = [0u8; 32 * 32];
        dir.serialize(&mut out);
        assert_eq!(&out[0..11], b"FOO     TXT");
        assert_eq!(&out[32..43], b"BAR     TXT");
    }
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rz_rust_os::test_panic_handler(info)
}
