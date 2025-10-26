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

    println!("Hello World{}", "!");
    rz_rust_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    let heap_value = Box::new(41);
    println!("heap_value at {:p}", heap_value);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    // --- demo: use in-repo FS on an in-memory mock device ---
    {
        use rz_rust_os::fs::fs::FileSystem;
        use rz_rust_os::fs::mock_device::MockDevice;

        static mut FS_BUF: [u8; 512 * 64] = [0u8; 512 * 64];
        unsafe {
            let buf = &mut FS_BUF[..];
            let mut dev = MockDevice::new(buf);
            // format the mock device as a FAT12 volume
            let sectors = dev.sector_count() as u16;
            FileSystem::format(&mut dev, sectors).expect("format failed");
            // mount
            let mut fs = FileSystem::mount(&mut dev).expect("mount failed");
            // write two files
            fs.write_file("FOOTXT", b"Hello from kernel").expect("write foo failed");
            fs.write_file("BARTXT", b"This contains the contents of the second file").expect("write bar failed");
            // read them back
            if let Ok(data) = fs.read_file("FOOTXT") {
                let s = core::str::from_utf8(&data).unwrap_or("<invalid utf8>");
                println!("FOO => {}", s);
            }
            if let Ok(data) = fs.read_file("BARTXT") {
                let s = core::str::from_utf8(&data).unwrap_or("<invalid utf8>");
                println!("BAR => {}", s);
            }
            // list root
            let list = fs.list_root();
            println!("Root dir entries: {}", list.len());
            for e in list.iter() {
                let mut nm = [0u8; 11];
                nm[0..8].copy_from_slice(&e.name);
                nm[8..11].copy_from_slice(&e.ext);
                // print raw bytes as string
                if let Ok(s) = core::str::from_utf8(&nm) {
                    println!(" - {} (cluster {}, size {})", s, e.start_cluster, e.file_size);
                }
            }
        }
    }

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));
    
    #[cfg(test)]
    test_main();
    
    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

#[allow(dead_code)]
async fn async_number() -> u32 {
    42
}

#[allow(dead_code)]
async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
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