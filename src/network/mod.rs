pub mod device;
pub mod e1000;
pub mod buf;
pub mod ethernet;
pub mod arp;
pub mod ipv4;
pub mod icmp;
pub mod udp;
pub mod sockets;
pub mod checksums;
pub mod network;

// Callers can use crate::network::init() and crate::network::poll().
pub use network::{init, poll};