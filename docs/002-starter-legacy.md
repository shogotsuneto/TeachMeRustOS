# Tiny Rust OS — starter kit (Rust + QEMU)

This is a minimal, study‑friendly kernel written in Rust that boots in QEMU and prints a message using the VGA text buffer. It’s intentionally tiny so you can learn both OS fundamentals **and** Rust at the same time.

---

## What you’ll build

* A `no_std`, `no_main` Rust binary that serves as your kernel
* Bootable disk image via `bootimage`
* Runs on a virtual machine (QEMU) — no need to touch your real machine

---

## Prerequisites (once per machine)

1. **Rust nightly & components**

```bash
rustup toolchain install nightly
rustup override set nightly
rustup component add rust-src llvm-tools-preview
```

2. **Bootimage tool** (builds a bootable disk image):

```bash
cargo install bootimage
```

3. **QEMU** (virtual machine):

* macOS: `brew install qemu`
* Ubuntu/Debian: `sudo apt-get install qemu-system-x86`
* Fedora: `sudo dnf install qemu-system-x86`
* Windows (MSYS/Chocolatey): install the QEMU package

---

## Project layout

```
rustos/
├─ Cargo.toml
├─ rust-toolchain.toml
├─ .cargo/
│  └─ config.toml
└─ src/
   ├─ main.rs
   └─ vga_buffer.rs
```

---

## `Cargo.toml`

````toml
[package]
name = "rustos"
version = "0.1.0"
edition = "2021"

[dependencies]
spin = "0.9"
x86_64 = "0.15"
bootloader = "0.9.23"

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
````

---

## `rust-toolchain.toml`

```toml
[toolchain]
channel = "nightly"
components = ["rust-src", "llvm-tools-preview"]
```

---

## `.cargo/config.toml`


```toml
[build]
target = "x86_64-unknown-none"

[target.x86_64-unknown-none]
runner = "bootimage runner"

[unstable]
build-std = ["core", "compiler_builtins", "alloc"]
build-std-features = ["compiler-builtins-mem"]
```

## `src/main.rs`

```rust
#![no_std]
#![no_main]

mod vga_buffer;

use core::panic::PanicInfo;
use x86_64::instructions::hlt;
use bootloader::BootInfo; // v0.9 API

#[no_mangle]
pub extern "C" fn _start(_boot_info: &'static BootInfo) -> ! {
    vga_buffer::clear_screen();
    vga_buffer::printk("Hello from a tiny Rust kernel!\n");
    vga_buffer::printk("Booted via bootloader v0.9 + bootimage.\n");
    loop { hlt(); }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    vga_buffer::printk("PANIC: something went wrong\n");
    loop { hlt(); }
}
```

## `src/vga_buffer.rs`

```rust
use core::fmt::{self, Write};
use spin::Mutex;

// Minimal volatile wrapper so we don't depend on an external crate.
#[repr(transparent)]
pub struct Volatile<T> {
    value: T,
}
impl<T> Volatile<T> {
    #[inline]
    pub fn read(&self) -> T
    where
        T: Copy,
    {
        unsafe { core::ptr::read_volatile(&self.value) }
    }
    #[inline]
    pub fn write(&mut self, val: T) {
        unsafe { core::ptr::write_volatile(&mut self.value, val) }
    }
}

// VGA text mode buffer is memory‑mapped here in real PCs/VMs
const BUFFER_ADDR: usize = 0xb8000;
const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

#[repr(transparent)]
#[derive(Copy, Clone)]
struct ColorCode(u8); // light gray on black (0x07)

#[repr(C)]
#[derive(Copy, Clone)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                });
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let ch = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(ch);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

static WRITER: Mutex<Option<Writer>> = Mutex::new(None);

fn writer() -> spin::MutexGuard<'static, Option<Writer>> {
    let mut guard = WRITER.lock();
    if guard.is_none() {
        *guard = Some(Writer {
            column_position: 0,
            color_code: ColorCode(0x07),
            buffer: unsafe { &mut *(BUFFER_ADDR as *mut Buffer) },
        });
    }
    guard
}

pub fn printk(s: &str) {
    use core::fmt::Write;
    if let Some(w) = &mut *writer() {
        let _ = w.write_str(s);
    }
}

pub fn clear_screen() {
    if let Some(w) = &mut *writer() {
        for row in 0..BUFFER_HEIGHT {
            w.clear_row(row);
        }
        w.column_position = 0;
    }
}
```

---

## Build & run
From the project root:
```bash
cargo bootimage         # builds a bootable image via `bootimage`
cargo run               # launches QEMU with that image (thanks to runner)
````

You should see a QEMU window with:

```
Hello from a tiny Rust kernel!
You are learning OS + Rust at once. ✨
```

---

## Where to go next

1. **Interrupts**: add the IDT (interrupt descriptor table) and enable keyboard input.
2. **Memory management**: set up paging; map a frame allocator; print physical/virtual addresses.
3. **Allocator**: bring in `alloc` and a simple bump allocator; try dynamic data structures.
4. **Timers & tasks**: PIT/APIC timer, cooperative task executor.
5. **Drivers**: keyboard, serial (COM1) for logging, maybe a basic framebuffer.

Each step is bite‑sized and perfect for learning Rust’s ownership + lifetimes alongside OS internals.

---

## Tips

* Keep your commits tiny and runnable in QEMU.
* Prefer `loop { hlt(); }` when idle to avoid pegging a host CPU core.
* If nightly breaks, pin to a specific date in `rust-toolchain.toml` (e.g., `"nightly-2025-08-01"`).

Happy hacking! 🦀🧠
