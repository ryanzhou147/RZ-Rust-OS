RZ-Rust-OS

Implemented features (in chronological order):

- Bare bones / freestanding Rust binary (crate attributes, no_std) (src/main.rs)
- Minimal kernel / bootable image (kernel entry and bootloader wiring) (src/main.rs)
- VGA text mode printing helper (safe VGA wrapper) (src/vga_buffer.rs)
- Unit & integration testing in no_std (custom test runner / test harness) (src/lib.rs, tests/)
- Interrupt descriptor table and handlers (interrupts, exceptions) (src/interrupts.rs)
- Global descriptor table and stacks for exceptions (src/gdt.rs)
- PIC and keyboard hardware interrupt handling (src/interrupts.rs, src/task/keyboard.rs)
- Paging and memory mapping (virtual memory init) (src/memory.rs)
- Heap allocation support and allocator implementations (src/allocator.rs, src/allocator/*)
- Multitasking basics and executor (task module) (src/task/mod.rs, src/task/executor.rs)
- Async/await task support and simple executor (src/task/simple_executor.rs)
- BlockDevice trait (512B sector I/O abstraction) (src/fs/block_device.rs)
- Slice-backed and fixed-size mock devices for tests (src/fs/mock_device.rs)
- Boot sector (BPB) parser & serializer for FAT12 (src/fs/boot_sector.rs)
- FAT12 table reader/writer, alloc/free, chain traversal (src/fs/fat_table.rs)
- Root-directory parsing and directory operations (src/fs/directory.rs)
- High-level FileSystem API: mount/read/write/delete/list/format (src/fs/fs.rs)
- Kernel demo that formats an in-memory device and creates/reads files (src/main.rs)

TODOs:

- Implement a simple CLI for interacting with the kernel (command parsing, basic shell)
- Implement basic networking support (NIC driver + packet I/O stack)

