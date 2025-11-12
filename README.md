# 🦀 RZ Rust OS

A minimal Rust-based operating system kernel built from scratch.

---

## 🧰 Try It Out

### 1. Install QEMU

#### **Linux (Ubuntu / Debian)**

```bash
sudo apt update
sudo apt install qemu-system-x86
```

#### **macOS**

Install with [Homebrew](https://brew.sh/):

```bash
brew install qemu
```

#### **Windows**

1. Download the installer from [qemu.org/download](https://www.qemu.org/download/)
2. Add QEMU to your system PATH
3. Verify installation:

   ```bash
   qemu-system-x86_64 --version
   ```

---

### 2. Run the OS

#### **Linux / macOS**

```bash
sh -c 'set -e
URL="https://raw.githubusercontent.com/ryanzhou147/RZ-Rust-OS/main/rz_rust_os.bin"
if command -v curl >/dev/null 2>&1; then curl -fL -o rz_rust_os.bin "$URL"
elif command -v wget >/dev/null 2>&1; then wget -O rz_rust_os.bin "$URL"
else echo "Please install curl or wget" >&2; exit 1; fi
qemu-system-x86_64 -m 512 -serial stdio -drive format=raw,file=rz_rust_os.bin'
```
#### **Windows**

```bash
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/ryanzhou147/RZ-Rust-OS/main/rz_rust_os.bin" -OutFile "rz_rust_os.bin"
qemu-system-x86_64.exe -m 512 -serial stdio -drive format=raw,file=rz_rust_os.bin
```

---

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
- Simple shell for interacting with the filesystem (read, write, ls, delete) (src/task/keyboard.rs, src/task/shell.rs)

TODOs (in order of priority):

- Implement basic networking support (NIC driver + packet I/O stack)
- Option to show physical/virtual memory locations of saved files
- Implement frame buffer graphics driver
- Implement directory tree for filesystem
- Implement mouse driver


