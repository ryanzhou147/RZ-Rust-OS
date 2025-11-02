#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rz_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use rz_rust_os::network::checksums::{ipv4_checksum, udp_checksum, verify_udp_checksum};
use alloc::vec::Vec;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    rz_rust_os::init();
    use rz_rust_os::allocator;
    use rz_rust_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    // Initialize memory and heap so tests can use `alloc` (Vec, Box, etc.).
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization in tests failed");


    test_main();
    loop {}
}

#[test_case]

fn checksum_types() {
    let _ = ipv4_checksum(&[0u8; 20]);
    let _ = udp_checksum([0,0,0,0], [0,0,0,0], &[]);
}

#[test_case]
fn ipv4_checksum_roundtrip() {
    // construct a minimal IPv4 header with zeros in checksum field
    let mut hdr = [0u8; 20];
    hdr[0] = 0x45; // version=4, ihl=5
    hdr[1] = 0; // tos
    hdr[2..4].copy_from_slice(&0u16.to_be_bytes()); // total len
    hdr[4..6].copy_from_slice(&0u16.to_be_bytes()); // id
    hdr[6..8].copy_from_slice(&0u16.to_be_bytes()); // flags/frag
    hdr[8] = 64; // ttl
    hdr[9] = 6; // proto TCP
    hdr[10] = 0; hdr[11] = 0; // checksum zero
    hdr[12..16].copy_from_slice(&[192,168,0,1]);
    hdr[16..20].copy_from_slice(&[192,168,0,2]);

    let c = ipv4_checksum(&hdr);
    hdr[10..12].copy_from_slice(&c.to_be_bytes());

    // raw sum including checksum should be 0xffff
    let mut sum: u32 = 0;
    for i in (0..hdr.len()).step_by(2) {
        let word = u16::from_be_bytes([hdr[i], hdr[i+1]]) as u32;
        sum = sum.wrapping_add(word);
    }
    while (sum >> 16) != 0 { sum = (sum & 0xffff) + (sum >> 16); }
    assert_eq!(sum as u16, 0xffff);
}

#[test_case]
fn udp_checksum_roundtrip() {
    let src = [10,0,0,1];
    let dst = [10,0,0,2];
    // UDP header: src port, dst port, len, checksum
    let mut udp = Vec::new();
    udp.extend_from_slice(&1234u16.to_be_bytes());
    udp.extend_from_slice(&4321u16.to_be_bytes());
    // placeholder length
    let payload = b"hello";
    let udp_len = (8 + payload.len()) as u16;
    udp.extend_from_slice(&udp_len.to_be_bytes());
    udp.extend_from_slice(&0u16.to_be_bytes()); // checksum zero
    udp.extend_from_slice(payload);

    let c = udp_checksum(src, dst, &udp);
    // write checksum into packet
    udp[6..8].copy_from_slice(&c.to_be_bytes());

    // compute raw sum over pseudo-header + udp packet, should be 0xffff
    let mut sum: u32 = 0;
    // src
    for i in 0..2 { sum = sum.wrapping_add(u16::from_be_bytes([src[i*2], src[i*2+1]]) as u32); }
    // dst
    for i in 0..2 { sum = sum.wrapping_add(u16::from_be_bytes([dst[i*2], dst[i*2+1]]) as u32); }
    // zero + proto
    sum = sum.wrapping_add(0);
    sum = sum.wrapping_add(17);
    sum = sum.wrapping_add(udp_len as u32);

    for i in (0..udp.len()).step_by(2) {
        let hi = udp[i];
        let lo = if i+1 < udp.len() { udp[i+1] } else { 0 };
        sum = sum.wrapping_add(u16::from_be_bytes([hi, lo]) as u32);
    }
    while (sum >> 16) != 0 { sum = (sum & 0xffff) + (sum >> 16); }
    assert_eq!(sum as u16, 0xffff);
}

#[test_case]
fn udp_verify_roundtrip() {
    let src = [10,0,0,1];
    let dst = [10,0,0,2];
    // UDP header: src port, dst port, len, checksum
    let mut udp = Vec::new();
    udp.extend_from_slice(&1234u16.to_be_bytes());
    udp.extend_from_slice(&4321u16.to_be_bytes());
    let payload = b"hello"; // odd-length payload
    let udp_len = (8 + payload.len()) as u16;
    udp.extend_from_slice(&udp_len.to_be_bytes());
    udp.extend_from_slice(&0u16.to_be_bytes()); // checksum zero
    udp.extend_from_slice(payload);

    let c = udp_checksum(src, dst, &udp);
    udp[6..8].copy_from_slice(&c.to_be_bytes());

    assert!(verify_udp_checksum(src, dst, &udp));
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rz_rust_os::test_panic_handler(info)
}
