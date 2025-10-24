:::mermaid
flowchart TD

    %% =====================
    %% Root system components
    %% =====================
    subgraph Kernel["**kernel_main** <br>(no_std environment)"]
        A["ALLOCATOR <br><br>(heap allocator)<br><br>➡ static ALLOCATOR: Locked<FixedSizeBlockAllocator>"]
    end


    subgraph FS["Filesystem Module (crate::fs)"]
        direction TB

        %% Core modules
        B["**block_device.rs**<br> Defines the BlockDevice trait<br>→ read_sector(), write_sector()"]
        C["**mock_device.rs**<br> Implements BlockDevice for testing<br>→ MockDevice with in-memory buffer"]
        D["**boot_sector.rs**<br> Parses FAT12 boot sector<br>→ BootSector struct + parse() / serialize()"]
        E["**fat_constants.rs**<br> Contains FAT12 constants<br>→ BYTES_PER_SECTOR, FAT12_MAX_CLUSTERS, etc."]
        F["**fat_table.rs**<br> Manages FAT table (cluster chains)<br>→ alloc_cluster(), write_entry(), get_chain()"]
        G["**directory.rs**<br> Manages root directory entries<br>→ Directory & DirectoryEntry structs"]
        H["**fs.rs**<br> High-level FileSystem interface<br>→ format(), mount(), read_file(), write_file(), delete(), list_root()"]

    end

    %% Relationships between modules
    H -->|Uses| B
    H -->|Uses| D
    H -->|Uses| F
    H -->|Uses| G
    H -->|Uses| E

    F -->|Reads/Writes clusters via| B
    G -->|Reads/Writes entries via| B
    D -->|Defines boot parameters for| F
    D -->|Defines root dir offsets for| G

    C -->|Implements| B
    H -->|Can use| C

    %% Allocator connection
    FS --> spacer["Uses alloc crate"]
    spacer --> |"(Vec, Box, etc.)"| A
    A -->|Provides heap memory to| FS

    %% Optional test/demo environment
    subgraph TestEnv["Testing / Example Use"]
        direction TB
        T["Example code<br>(creates MockDevice, formats, mounts, reads/writes files)"]
    end

    T -->|Creates & formats| C
    T -->|Mounts| H
    T -->|Prints directory listing| G

:::