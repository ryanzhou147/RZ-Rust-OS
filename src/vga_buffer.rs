use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

lazy_static! {
    /// A global `Writer` instance that can be used for printing to the VGA text buffer.
    ///
    /// Used by the `print!` and `println!` macros.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::LightCyan, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        current_row: 0,
        reserved_top_rows: 0,
    });
}

/// The standard color palette in VGA text mode.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// A combination of a foreground and a background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a new `ColorCode` with the given foreground and background colors.
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A screen character in the VGA text buffer, consisting of an ASCII character and a `ColorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// The height of the text buffer (normally 25 lines).
const BUFFER_HEIGHT: usize = 25;
/// The width of the text buffer (normally 80 columns).
const BUFFER_WIDTH: usize = 80;

/// A structure representing the VGA text buffer.
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// A writer type that allows writing ASCII bytes and strings to an underlying `Buffer`.
///
/// Wraps lines at `BUFFER_WIDTH`. Supports newline characters and implements the
/// `core::fmt::Write` trait.
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
    /// The current row where the next character will be written. When writing
    /// from the top, this starts at 0 and increments on newlines. If it
    /// reaches BUFFER_HEIGHT, the buffer scrolls up and the last row is used.
    current_row: usize,
    /// Number of top rows reserved as a fixed header. These rows will not be
    /// scrolled when the text buffer advances.
    reserved_top_rows: usize,
}

impl Writer {
    /// Writes an ASCII byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\t' => {
                // expand tab to 4 spaces
                for _ in 0..4 {
                    self.write_byte(b' ');
                }
            }
            0x08 => {
                // backspace: move cursor back one and clear that cell
                if self.column_position <= 2 {
                    return;
                }
                self.column_position -= 1;
                let row = self.current_row;
                let col = self.column_position;
                let blank = ScreenChar {
                    ascii_character: b' ',
                    color_code: self.color_code,
                };
                self.buffer.chars[row][col].write(blank);
            }
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = self.current_row;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    /// Writes the given ASCII string to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character. Does **not**
    /// support strings with non-ASCII characters, since they can't be printed in the VGA text
    /// mode.
    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte, newline, tab, or backspace
                0x20..=0x7e | b'\n' | b'\t' | 0x08 => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// Shifts all lines one line up and clears the last row.
    fn new_line(&mut self) {
        // Advance to the next row. If we reach the bottom, scroll the buffer
        // up by one row but keep the reserved top rows untouched.
        if self.current_row + 1 < BUFFER_HEIGHT {
            self.current_row += 1;
        } else {
            // shift rows up starting from reserved_top_rows so the header stays
            // fixed at the top.
            let start = self.reserved_top_rows;
            for row in start..(BUFFER_HEIGHT - 1) {
                for col in 0..BUFFER_WIDTH {
                    let character = self.buffer.chars[row + 1][col].read();
                    self.buffer.chars[row][col].write(character);
                }
            }
            self.clear_row(BUFFER_HEIGHT - 1);
            self.current_row = BUFFER_HEIGHT - 1;
        }
        self.column_position = 0;
    }

    /// Clears a row by overwriting it with blank characters.
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    /// Writes the given ASCII string to a specific row starting at column 0.
    /// Characters beyond the row width are truncated. This does not affect the
    /// normal cursor (`column_position`) used for subsequent writes.
    pub fn write_row_at(&mut self, row: usize, s: &str) {
        if row >= BUFFER_HEIGHT { return; }
        // clear the row first
        self.clear_row(row);
        let mut col = 0usize;
        for byte in s.bytes() {
            if col >= BUFFER_WIDTH { break; }
            // printable ASCII only
            let ch = match byte {
                0x20..=0x7e => byte,
                _ => 0xfe,
            };
            let sc = ScreenChar { ascii_character: ch, color_code: self.color_code };
            self.buffer.chars[row][col].write(sc);
            col += 1;
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Like the `print!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {     // new
        WRITER.lock().write_fmt(args).unwrap();
    });}

/// Write a string to the top row (row 0) of the VGA text buffer.
pub fn set_top_line(s: &str) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_row_at(0, s);
    });
}

/// Reset the writer cursor to the top-left of the screen. Subsequent writes will
/// begin at row 0, column 0.
pub fn reset_to_top() {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        let mut w = WRITER.lock();
        // Reset to first writable row (i.e., after reserved header rows)
        w.current_row = w.reserved_top_rows;
        w.column_position = 0;
    });
}

/// Hide the hardware text-mode cursor (prevents the blinking underline/block).
pub fn hide_hardware_cursor() {
    use x86_64::instructions::port::Port;

    // VGA CRT Controller registers: index 0x0A is cursor start. Setting bit 5
    // (0x20) disables the cursor. We write the index to port 0x3D4 and the
    // value to 0x3D5.
    unsafe {
        let mut index = Port::new(0x3D4u16);
        let mut data = Port::new(0x3D5u16);
        // select cursor start register
        index.write(0x0Au8);
        // set disable bit
        data.write(0x20u8);
    }
}

/// Set the starting row where subsequent writes should begin. If `row` is
/// greater than or equal to `BUFFER_HEIGHT` this is a no-op.
pub fn set_start_row(row: usize) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        if row >= BUFFER_HEIGHT { return; }
        let mut w = WRITER.lock();
        // Do not allow starting above the reserved header rows
        w.current_row = core::cmp::max(row, w.reserved_top_rows);
        w.column_position = 0;
    });
}

/// Set the text of a specific row (0-based). This clears the row first and
/// writes the provided string (truncated to the row width).
pub fn set_row(row: usize, s: &str) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_row_at(row, s);
    });
}

/// Reserve the top `n` rows as a fixed header. These rows will not be
/// scrolled when the console advances. Call this before writing header rows
/// and resetting the writer pointer.
pub fn set_reserved_top_rows(n: usize) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        let mut w = WRITER.lock();
        w.reserved_top_rows = core::cmp::min(n, BUFFER_HEIGHT);
    });
}

/// Ensure the header is at the very top of the screen. If the first row is
/// blank but the second row contains text, move the second row into the first
/// row and clear the second. This fixes cases where an intermittent blank row
/// appears above the header due to early prints or race conditions.
pub fn normalize_header() {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        let mut w = WRITER.lock();
        // Check if row 0 is blank and row 1 has any non-space char
        let mut row0_blank = true;
        for col in 0..BUFFER_WIDTH {
            let c = w.buffer.chars[0][col].read();
            if c.ascii_character != b' ' { row0_blank = false; break; }
        }
        if !row0_blank { return; }
        let mut row1_blank = true;
        for col in 0..BUFFER_WIDTH {
            let c = w.buffer.chars[1][col].read();
            if c.ascii_character != b' ' { row1_blank = false; break; }
        }
        if row1_blank { return; }
        // move row1 -> row0
        let color = w.color_code;
        for col in 0..BUFFER_WIDTH {
            let ch = w.buffer.chars[1][col].read();
            w.buffer.chars[0][col].write(ch);
            // clear row1
            w.buffer.chars[1][col].write(ScreenChar { ascii_character: b' ', color_code: color });
        }
    });
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Some test string that fits on a single line";
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[1][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}
