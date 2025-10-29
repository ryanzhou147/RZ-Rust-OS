:::mermaid
flowchart TD

    %% =====================
    %% Root system components
    %% =====================
    subgraph Kernel["**kernel_main** <br>(no_std environment)"]
        A["ALLOCATOR <br><br>(heap allocator)<br>➡ provides `alloc::Vec`, `Box`"]
    end

   %% Networking root
    subgraph Net["**crate::network**"]
        direction TB

        DEV["**device.rs**<br>NetworkDevice<br> •transmit() / receive() / handle_interrupt()"]
        E1000["**e1000.rs**<br> •E1000::init() / interrupt_handler() (impl NetworkDevice)"]
        BUF["**buf.rs**<br> •PacketBuf::push_bytes()"]
        ETH["**ethernet.rs**<br> •parse_eth_header() / build_eth_frame()"]
        ARP["**arp.rs**<br> •ArpCache::lookup()/insert()<br> •handle_arp_packet()"]
        IPV4["**ipv4.rs**<br> •parse_ipv4_header() / build_ipv4_packet()"]
        ICMP["**icmp.rs**<br> •handle_icmp()"]
        UDP["**udp.rs**<br> •UdpSocket::send_to() / recv_from()"]
        SOCK["**sockets.rs**<br> •SocketWaker::wake() / register()"]
        CS["**checksums.rs**<br> •ipv4_checksum() / udp_checksum()"]
        TOP["**mod.rs**<br> •net::init() / net::poll()"]
    end

    %% Relationships labelled with function-level arrows (concise)
    Kernel -->|provides heap to| Net

  %% driver boundary
    DEV -->|"implemented by"| E1000
    E1000 -->|"interrupt -> calls"| TOP
    TOP -->|"calls device.receive() / device.transmit()"| DEV

    %% transmit path (high-level)
    UDP -->|"send_to() -> calls"| IPV4
    IPV4 -->|"build_ipv4_packet() -> calls"| CS
    IPV4 -->|"build_ipv4_packet() -> then calls"| ETH
    ETH -->|"build_eth_frame() -> passes bytes to"| DEV

    %% receive path (high-level)
    DEV -->|"receive() -> yields bytes to"| ETH
    ETH -->|"parse_eth_header() -> demux to"| ARP
    ETH -->|"parse_eth_header() -> demux to"| IPV4

    %% L3 demux
    ARP -->|"handle_arp_packet() -> may return reply to"| DEV
    IPV4 -->|"parse_ipv4_header() -> demux to"| ICMP
    IPV4 -->|"parse_ipv4_header() -> demux to"| UDP

    %% UDP/ICMP handling
    ICMP -->|"handle_icmp() -> may build reply via"| IPV4
    UDP -->|"recv_from() -> enqueues to"| SOCK
    SOCK -->|"SocketWaker::wake() -> wakes tasks waiting on"| UDP

    %% buffer usage
    DEV -->|"RX/TX copy -> uses"| BUF
    BUF -->|"holds bytes for"| ETH

    %% checksums used where
    IPV4 -->|"uses"| CS
    UDP -->|"uses"| CS

    %% interrupt -> wakes sockets
    E1000 -->|"interrupt_handler() -> should call"| DEV
    DEV -->|"handle_interrupt() -> should call"| SOCK

    %% Tests / notes
    subgraph Tests["#[cfg(test)] compile tests"]
        T["validate API shapes: NetworkDevice, E1000, parsers, PacketBuf, UdpSocket"]
    end
    T --> DEV
    T --> E1000
    T --> ETH
    T --> BUF

    %% Notes
    %% note right of E1
    %%   TODOs:
    %%   - PCI/MMIO mapping
    %%   - Rx/Tx descriptor rings
    %%   - DMA-safe buffers
    %%   - real interrupt ack
    %% end note

    %% note right of SCK
    %%   Waker notes:
    %%   - Use `AtomicWaker` or kernel-safe equivalent
    %%   - Must be callable from IRQ context
    %% end note
:::
