extern crate alloc;
use alloc::vec::Vec;

// A tiny waker placeholder. In a kernel this should use an AtomicWaker-like
// implementation to wake tasks from interrupt context.
pub struct SocketWaker {
    // TODO: store waker / AtomicWaker here
}

impl SocketWaker {
    pub fn new() -> Self { SocketWaker {} }
    pub fn wake(&self) { /* TODO */ }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waker_new() {
        let _ = SocketWaker::new();
    }
}
