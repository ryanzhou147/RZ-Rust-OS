# RZ-Rust-OS

Bare-metal x86_64 operating system written from scratch in Rust. Features custom BIOS bootloader, virtual memory with heap allocation, FAT12 filesystem, and async task executor.

## Quick Start

**Install QEMU:**
```bash
# Linux
sudo apt install qemu-system-x86

# macOS
brew install qemu

# Windows: download from qemu.org/download and add to PATH
```

**Run:**
```bash
# Linux / macOS
curl -fLO https://raw.githubusercontent.com/ryanzhou147/RZ-Rust-OS/main/rz_rust_os.bin
qemu-system-x86_64 -m 512 -serial stdio -drive format=raw,file=rz_rust_os.bin

# Windows (PowerShell)
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/ryanzhou147/RZ-Rust-OS/main/rz_rust_os.bin" -OutFile "rz_rust_os.bin"
qemu-system-x86_64.exe -m 512 -serial stdio -drive format=raw,file=rz_rust_os.bin
```

## Architecture
```
src/
├── main.rs          # Kernel entry
├── vga_buffer.rs    # VGA text mode
├── interrupts.rs    # IDT, PIC, keyboard
├── gdt.rs           # Global descriptor table
├── memory.rs        # Paging, virtual memory
├── allocator/       # Heap allocators
├── task/            # Async executor, shell
└── fs/              # FAT12 filesystem
    ├── block_device.rs
    ├── boot_sector.rs
    ├── fat_table.rs
    ├── directory.rs
    └── fs.rs
```

## Implemented features (in chronological order):
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
- Simple shell for interacting with the filesystem (read, write, ls, delete) (src/task/keyboard.rs, src/task/shell.rs)
- Implement basic networking support (NIC driver + packet I/O stack)
- Option to show physical/virtual memory locations of saved files
- Implement frame buffer graphics driver
- Implement directory tree for filesystem
- Implement mouse driver
