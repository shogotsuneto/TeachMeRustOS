#![no_std]
#![no_main]

mod vga_buffer;
mod serial;

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use x86_64::instructions::hlt;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial::init();
    serial::println("kernel: entered kernel_main");

    if let Some(fb) = boot_info.framebuffer.as_mut() {
        serial::println("kernel: framebuffer present -> skip VGA writes");
        let info = fb.info();
        let buf = fb.buffer_mut();
        let w = info.width.min(200);
        let h = info.height.min(100);
        let bpp = info.bytes_per_pixel;
        let stride = info.stride;

        for y in 0..h {
            for x in 0..w {
                let i = (y * stride + x) * bpp;
                if i + (bpp - 1) < buf.len() {
                    buf[i] = 0xFF;
                    if bpp > 1 { buf[i + 1] = 0x80; }
                    if bpp > 2 { buf[i + 2] = 0x00; }
                    if bpp > 3 { buf[i + 3] = 0x00; }
                }
            }
        }
        serial::println("kernel: drew rectangle");
    } else {
        // Only use VGA text mode if no framebuffer is available
        serial::println("kernel: no framebuffer -> using VGA text");
        vga_buffer::clear_screen();
        vga_buffer::printk("Hello from a modern Rust kernel (VGA text)!\n");
    }

    serial::println("kernel: hlt loop");
    loop { hlt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println("PANIC");
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        serial::println(s);
    }
    loop { hlt(); }
}
