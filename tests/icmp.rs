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

use rz_rust_os::network::icmp::{handle_icmp};
use rz_rust_os::network::checksums::compute_ck16;
use rz_rust_os::network::ipv4::Ipv4Header;

use alloc::vec::Vec;
use alloc::vec;

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
fn echo_request_reply_roundtrip() {
    // build an echo request: type 8, code 0, checksum 0, id=0x1234, seq=1, payload="abc"
    let mut req: Vec<u8> = vec![];
    req.push(8u8); req.push(0u8);
    req.extend_from_slice(&[0u8, 0u8]);
    req.extend_from_slice(&0x1234u16.to_be_bytes());
    req.extend_from_slice(&1u16.to_be_bytes());
    req.extend_from_slice(b"abc");
    // compute checksum
    let mut tmp = req.clone(); tmp[2]=0; tmp[3]=0;
    let ck = compute_ck16(&tmp);
    req[2..4].copy_from_slice(&ck.to_be_bytes());

    let hdr = Ipv4Header { src: [0,0,0,0], dst: [0,0,0,0], proto: 1, header_len: 20, total_len: 0 };
    let reply = handle_icmp(&hdr, &req).expect("should reply");
    // reply should be type 0, same id/seq and payload
    assert_eq!(reply[0], 0u8);
    assert_eq!(&reply[4..6], &0x1234u16.to_be_bytes());
    assert_eq!(&reply[6..8], &1u16.to_be_bytes());
    assert_eq!(&reply[8..], b"abc");
    // checksum must be correct
    let mut tmp2 = reply.clone(); tmp2[2]=0; tmp2[3]=0;
    let ck2 = compute_ck16(&tmp2);
    assert_eq!(ck2, u16::from_be_bytes([reply[2], reply[3]]));
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
rz_rust_os::test_panic_handler(info)
}
