use crate::{print, println};
use alloc::{string::String, vec::Vec};
use crate::task::keyboard::try_pop_key;
use crate::fs::directory::DirectoryEntry;
use crate::fs::block_device::BlockDevice;
use crate::fs::fs::FileSystem;
use crate::fs::mock_device::MockDevice;


fn format_8_3(name: &str) -> String {
    let up = name.to_ascii_uppercase();
    let mut parts = up.splitn(2, '.');
    let base = parts.next().unwrap_or("");
    let ext = parts.next().unwrap_or("");
    let mut base_buf = [b' '; 8];
    for (i, &b) in base.as_bytes().iter().take(8).enumerate() {
        base_buf[i] = b;
    }
    let mut ext_buf = [b' '; 3];
    for (i, &b) in ext.as_bytes().iter().take(3).enumerate() {
        ext_buf[i] = b;
    }
    let mut s = String::new();
    for &b in base_buf.iter() { s.push(b as char); }
    for &b in ext_buf.iter() { s.push(b as char); }
    s
}

fn display_entry_name(e: &DirectoryEntry) -> String {
    let base = core::str::from_utf8(&e.name).unwrap_or("").trim_end_matches(' ');
    let ext = core::str::from_utf8(&e.ext).unwrap_or("").trim_end_matches(' ');
    if ext.is_empty() {
        String::from(base)
    } else {
        let mut s = String::new();
        s.push_str(base);
        s.push('.');
        s.push_str(ext);
        s
    }
}

/// Drain any queued keypresses up to (but not including) a newline and
/// return them as a String. If no characters are available, returns an
/// empty string.
pub fn flush_keypresses() {
    let mut s = String::new();
    while let Some(c) = try_pop_key() {
        if c == '\n' || c == '\r' {
            break;
        }
        s.push(c);
    }
    // Call the registered shell input handler (if any) with the assembled line.
    // This intentionally uses a simple direct call so the keyboard flush triggers
    // shell command execution immediately.
    shell_input(&s);
}

// A plain global pointer to the registered FileSystem. We intentionally avoid
// OnceCell/Mutex/etc per your request; `new()` must be called early (single-
// threaded init) with a leaked `'static` FileSystem reference.
static mut SHELL_FS_PTR: *mut FileSystem<'static, MockDevice<'static>> = core::ptr::null_mut();

/// Register a 'static FileSystem for the shell to use. Call this once during
/// early boot after you have created/mounted a FileSystem with a 'static
/// lifetime (for example via Box::leak).
pub fn new(fs: &'static mut FileSystem<'static, MockDevice<'static>>) {
    unsafe {
        SHELL_FS_PTR = fs as *mut _;
    }
}

/// Execute a single input line against the registered FileSystem. If no
/// FileSystem was registered, this prints an error.
pub fn shell_input(s: &str) {
    let line_trim = s.trim();
    if line_trim.is_empty() { return; }
    let mut parts = line_trim.split_whitespace();
    let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
    unsafe {
        if SHELL_FS_PTR.is_null() {
            println!("shell: no filesystem registered");
            return;
        }
        let fs: &mut FileSystem<'static, MockDevice<'static>> = &mut *SHELL_FS_PTR;
        match cmd.as_str() {
            "help" => {
                println!("Commands: help, ls, read <name>, write <name> <text>, delete <name>");
            }
            "ls" => {
                let list = fs.list_root();
                for e in list.iter() {
                    let name = display_entry_name(e);
                    println!("{}\t{} bytes", name, e.file_size);
                }
            }
            "read" => {
                if let Some(name) = parts.next() {
                    let name11 = format_8_3(name);
                    match fs.read_file(&name11) {
                        Ok(data) => {
                            if let Ok(s) = core::str::from_utf8(&data) {
                                println!("{}", s);
                            } else {
                                let mut out = String::new();
                                for b in data.iter() { out.push_str(&alloc::format!("{:02x}", b)); }
                                println!("{}", out);
                            }
                        }
                        Err(e) => println!("read error: {:?}", e),
                    }
                } else {
                    println!("usage: read <NAME>");
                }
            }
            "write" => {
                if let Some(name) = parts.next() {
                    let rest: Vec<&str> = parts.collect();
                    let data = rest.join(" ");
                    let name11 = format_8_3(name);
                    match fs.write_file(&name11, data.as_bytes()) {
                        Ok(()) => println!("wrote {} bytes", data.len()),
                        Err(e) => println!("write error: {:?}", e),
                    }
                } else {
                    println!("usage: write <NAME> <TEXT>");
                }
            }
            "delete" => {
                if let Some(name) = parts.next() {
                    let name11 = format_8_3(name);
                    match fs.delete(&name11) {
                        Ok(()) => println!("deleted {}", name),
                        Err(e) => println!("delete error: {:?}", e),
                    }
                } else {
                    println!("usage: delete <NAME>");
                }
            }
            other => {
                println!("unknown command: {}", other);
            }
        }
    }
}