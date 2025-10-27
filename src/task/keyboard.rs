use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{
    stream::{Stream, StreamExt},
    task::AtomicWaker,
};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1, KeyCode};
use spin::Mutex;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crate::{print, println};
use crate::task::shell::flush_keypresses;

/// Stores incoming keyboard scancodes (from interrupt handler)
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

/// Stores decoded characters (typed keys)
static KEYPRESS_BUFFER: OnceCell<Mutex<ArrayQueue<char>>> = OnceCell::uninit();

/// Used to wake the keyboard task when a new scancode arrives
static WAKER: AtomicWaker = AtomicWaker::new();

/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
    }
}

/// Stream of keyboard scancodes
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // Try immediately first
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        // Otherwise, register waker and check again
        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

/// The main async keyboard task — reads scancodes, decodes keys,
/// prints them, and fills the KEYPRESS_BUFFER.
pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    // Ensure the keypress buffer exists before we start consuming keys.
    KEYPRESS_BUFFER
        .try_init_once(|| Mutex::new(ArrayQueue::new(256)))
        .ok();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => {
                        // Add every key to queue
                        keypresses_queue(character);

                        // Echo to screen
                        print!("{}", character);

                        // If Enter pressed, notify shell to flush
                        if character == '\n' || character == '\r' {
                            flush_keypresses();
                        }
                    }
                    DecodedKey::RawKey(key) => {
                        // Map control keys like Backspace and Tab to characters
                        match key {
                            KeyCode::Backspace => {
                                // Use ASCII BS so consumers can handle it
                                keypresses_queue('\x08');
                                // Erase last character on echo
                                print!("backspacing");
                            }
                            KeyCode::Tab => {
                                keypresses_queue('\t');
                                print!("\t");
                            }
                            // Some scancode sets may produce Enter as a RawKey variant
                            // but we already handle Enter when decoded to Unicode '\n'.
                            other => {
                                // Fallback: print debug for other non-unicode keys
                                print!("{:?}", other);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Adds a single keypress to the global queue
fn keypresses_queue(c: char) {
    if let Ok(buf_cell) = KEYPRESS_BUFFER.try_get() {
        let buffer = buf_cell.lock();
        if c == '\x08' {
            // Backspace: drop last buffered character if any
            let _ = buffer.pop();
        } else {
            let _ = buffer.push(c); // ignore if full
        }
    }
}

/// Try to pop a single keypress character from the internal buffer.
/// Returns None if the buffer is empty or not initialized.
pub fn try_pop_key() -> Option<char> {
    if let Ok(buf_cell) = KEYPRESS_BUFFER.try_get() {
        let buffer = buf_cell.lock();
        buffer.pop()
    } else {
        None
    }
}

