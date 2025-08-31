use core::fmt::{self, Write};
use spin::Mutex;

#[repr(transparent)]
pub struct Volatile<T> {
    value: T,
}
impl<T> Volatile<T> {
    #[inline]
    pub fn read(&self) -> T where T: Copy {
        unsafe { core::ptr::read_volatile(&self.value) }
    }
    #[inline]
    pub fn write(&mut self, val: T) {
        unsafe { core::ptr::write_volatile(&mut self.value, val) }
    }
}

const BUFFER_ADDR: usize = 0xb8000;
const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

#[repr(transparent)]
#[derive(Copy, Clone)]
struct ColorCode(u8);

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
                if self.column_position >= BUFFER_WIDTH { self.new_line(); }
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
        let blank = ScreenChar { ascii_character: b' ', color_code: self.color_code };
        for col in 0..BUFFER_WIDTH { self.buffer.chars[row][col].write(blank); }
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() { self.write_byte(byte); }
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
    if let Some(w) = &mut *writer() { let _ = w.write_str(s); }
}

pub fn clear_screen() {
    if let Some(w) = &mut *writer() {
        for row in 0..BUFFER_HEIGHT { w.clear_row(row); }
        w.column_position = 0;
    }
}
