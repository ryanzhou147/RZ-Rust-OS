#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rz_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use rz_rust_os::network::ipv4::{build_ipv4_packet, parse_ipv4_header};

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    rz_rust_os::init();

    test_main();
    loop {}
}

#[test_case]
fn build_and_parse_roundtrip() {
    let src = [10u8,0,0,1];
    let dst = [10u8,0,0,2];
    let proto = 1u8; // ICMP
    let payload = [0xdeu8, 0xad, 0xbe, 0xef];
    let mut out = [0u8; 128];
    let len = build_ipv4_packet(src, dst, proto, &payload, &mut out).expect("build");
    let (hdr, pl) = parse_ipv4_header(&out[..len]).expect("parse");
    assert_eq!(hdr.src, src);
    assert_eq!(hdr.dst, dst);
    assert_eq!(hdr.proto, proto);
    assert_eq!(pl, &payload);
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rz_rust_os::test_panic_handler(info)
}
