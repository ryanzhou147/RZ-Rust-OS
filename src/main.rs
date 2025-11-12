#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rz_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};
use rz_rust_os::println;
use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use rz_rust_os::memory::{self, BootInfoFrameAllocator};
    use rz_rust_os::task::{Task, executor::Executor, keyboard};
    use rz_rust_os::allocator;
    use x86_64::VirtAddr;

    rz_rust_os::init();
    rz_rust_os::vga_buffer::set_reserved_top_rows(2);
    rz_rust_os::vga_buffer::hide_hardware_cursor();
    rz_rust_os::vga_buffer::set_row(0, "RZ Rust OS");
    rz_rust_os::vga_buffer::set_row(1, "Type \"help\" for a list of commands.");
    rz_rust_os::vga_buffer::normalize_header();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
    
    #[cfg(test)]
    test_main();
    
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    {
        // Demo: create a leaked mock device + filesystem, register it with the
        // shell, and run a few shell commands programmatically to demonstrate
        // ls/read/write/delete.
        use rz_rust_os::task::shell;
        use rz_rust_os::fs::mock_device::MockDevice;
        use rz_rust_os::fs::fs::FileSystem;

    // Header already written above via VGA helpers.

        // Allocate a boxed buffer for the device and leak it to get a 'static
        // slice for the MockDevice.
        let boxed_buf = Box::new([0u8; 512 * 64]);
        let leaked_buf: &'static mut [u8; 512 * 64] = Box::leak(boxed_buf);
        let leaked_slice: &'static mut [u8] = &mut leaked_buf[..];

        // Create a boxed MockDevice that borrows the leaked slice and leak it so
        // we have a 'static device to hand to FileSystem.
        let dev_box = Box::new(MockDevice::new(leaked_slice));
        let shell_dev: &'static mut MockDevice = Box::leak(dev_box);

        // Format and mount the filesystem on the leaked device.
        let sectors = shell_dev.sector_count() as u16;
        FileSystem::format(shell_dev, sectors).expect("format failed");
        let fs_box = Box::new(FileSystem::mount(shell_dev).expect("mount failed"));
        let shell_fs: &'static mut FileSystem<'static, MockDevice<'static>> = Box::leak(fs_box);


        // Register the filesystem with the shell.
        shell::new(shell_fs);
    }
    executor.run();
}


/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    rz_rust_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rz_rust_os::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}