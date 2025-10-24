:::mermaid
flowchart TD
    %% No-Heap Section
    subgraph NoHeap["No Heap Allocator"]
        GDT["**gdt.rs**<br>Sets up GDT and TSS<br>→ init_gdt(), load_tss()"]
        INT["**interrupts.rs**<br>IDT setup and ISR/IRQ handlers<br>→ init_idt(), register_handlers(), handle_interrupt()"]
        SERIAL["**serial.rs**<br>Serial I/O for debugging/logs<br>→ init_serial(), serial_print()"]
        VGA["**vga_buffer.rs**<br>Text output to screen<br>→ write_char(), write_string()"]
        MAIN["**main.rs**<br>Kernel entry point<br>→ init(), kernel_main()"]
    end

    %% Heap Section
    subgraph Heap["Heap Allocator"]
        TASK["**task/**<br>Task switching & scheduler<br>→ schedule(), switch_task(), add_task()"]
        MEMORY["**memory.rs**<br>Paging, frame allocator<br>→ init_memory(), allocate_frame(), deallocate_frame()"]
        ALLOC["**allocator.rs**<br>Heap allocator<br>→ ALLOCATOR: Locked<FixedSizeBlockAllocator>"]
    end

    %% Module Connections
    MAIN -->|Calls| GDT
    MAIN -->|Calls| INT
    MAIN -->|Initializes| SERIAL
    MAIN -->|Optional debug| VGA
    INT -->|Calls on interrupt| TASK
    INT -->|May log| SERIAL
    INT -->|May write| VGA
    TASK -->|Allocates memory| ALLOC
    TASK -->|Manages frames| MEMORY
    MEMORY -->|Uses heap for structures| ALLOC
    ALLOC -.->|Provides heap to| TASK
:::