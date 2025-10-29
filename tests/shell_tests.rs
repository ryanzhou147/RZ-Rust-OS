#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rz_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use alloc::boxed::Box;

use bootloader::{entry_point, BootInfo};
use rz_rust_os::allocator;
use rz_rust_os::memory::{self, BootInfoFrameAllocator};
use x86_64::VirtAddr;

use rz_rust_os::fs::mock_device::MockDevice;
use rz_rust_os::fs::fs::FileSystem;
use rz_rust_os::task::shell;
use rz_rust_os::task::shell::shell_input;
use rz_rust_os::fs::fs::FsError;

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

fn make_leaked_fs() -> &'static mut FileSystem<'static, MockDevice<'static>> {
    // allocate buffer on heap and leak to get 'static slice
    let boxed_buf = Box::new([0u8; 512 * 64]);
    let leaked_buf: &'static mut [u8; 512 * 64] = Box::leak(boxed_buf);
    let leaked_slice: &'static mut [u8] = &mut leaked_buf[..];
    // create device and leak
    let dev_box = Box::new(MockDevice::new(leaked_slice));
    let shell_dev: &'static mut MockDevice = Box::leak(dev_box);
    // format and mount
    let sectors = shell_dev.sector_count() as u16;
    FileSystem::format(shell_dev, sectors).expect("format failed");
    let fs_box = Box::new(FileSystem::mount(shell_dev).expect("mount failed"));
    let shell_fs: &'static mut FileSystem<'static, MockDevice<'static>> = Box::leak(fs_box);
    shell_fs
}

#[test_case]
fn shell_write_and_read() {
    let fs = make_leaked_fs();
    // register (use raw pointer to avoid borrow conflicts)
    shell::new(fs as *mut _);
    // write using shell
    shell_input("write test.txt hello_from_shell");
    // verify via fs
    let name11 = "TEST    TXT"; // 8 + 3
    match fs.read_file(name11) {
        Ok(data) => {
            let s = core::str::from_utf8(&data).unwrap_or("");
            assert!(s.contains("hello_from_shell"));
        }
        Err(e) => panic!("read failed: {:?}", e),
    }
}

#[test_case]
fn shell_list_directories() {
    let fs = make_leaked_fs();
    shell::new(fs as *mut _);
    shell_input("write a.txt a");
    shell_input("write b.txt b");
    let list = fs.list_root();
    // Expect at least two entries
    assert!(list.len() >= 2);
    // Check presence of A and B files
    let mut found_a = false;
    let mut found_b = false;
    for e in list.iter() {
        let mut nm = [0u8; 11];
        nm[0..8].copy_from_slice(&e.name);
        nm[8..11].copy_from_slice(&e.ext);
        if &nm == b"A       TXT" { found_a = true; }
        if &nm == b"B       TXT" { found_b = true; }
    }
    assert!(found_a && found_b, "expected files not found in root");
}

#[test_case]
fn shell_write_read_delete_read_again() {
    let fs = make_leaked_fs();
    shell::new(fs as *mut _);
    // write
    shell_input("write temp.txt secret");
    // read
    let name11 = "TEMP    TXT";
    let data = fs.read_file(name11).expect("read failed");
    let s = core::str::from_utf8(&data).unwrap_or("");
    assert!(s.contains("secret"));
    // delete
    shell_input("delete temp.txt");
    // reading again should fail
    match fs.read_file(name11) {
        Ok(_) => panic!("file still present after delete"),
        Err(e) => match e {
            FsError::NotFound => {},
            other => panic!("unexpected error: {:?}", other),
        }
    }
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rz_rust_os::test_panic_handler(info)
}
