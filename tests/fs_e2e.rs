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

use rz_rust_os::fs::mock_device::MockDevice;
use rz_rust_os::fs::fs::FileSystem;

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
fn e2e_format_write_read_list_delete() {
    static mut BUF: [u8; 512 * 64] = [0u8; 512 * 64];
    unsafe {
        let buf = &mut BUF[..];
        let mut dev = MockDevice::new(buf);
        // format device
        FileSystem::format(&mut dev, 2880).expect("format failed");
        // mount
        let mut fs = FileSystem::mount(&mut dev).expect("mount failed");
        // write a small file
        let data = b"hello world";
        fs.write_file("HELLO   TXT", data).expect("write failed");
        // read back
        let read = fs.read_file("HELLO   TXT").expect("read failed");
        assert_eq!(&read[..], &data[..]);
        // list
        let list = fs.list_root();
        assert!(list.len() >= 1);
        let mut found = false;
        for e in list.iter() {
            let mut nm = [0u8; 11];
            nm[0..8].copy_from_slice(&e.name);
            nm[8..11].copy_from_slice(&e.ext);
            if &nm == b"HELLO   TXT" { found = true; }
        }
        assert!(found, "file not found in root listing");
        // delete
        fs.delete("HELLO   TXT").expect("delete failed");
        let list2 = fs.list_root();
        // file should no longer be present
        let mut still = false;
        for e in list2.iter() {
            let mut nm = [0u8; 11];
            nm[0..8].copy_from_slice(&e.name);
            nm[8..11].copy_from_slice(&e.ext);
            if &nm == b"HELLO   TXT" { still = true; }
        }
        assert!(!still, "file still present after delete");
    }
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rz_rust_os::test_panic_handler(info)
}
