#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use x86_64::instructions::hlt;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // If a framebuffer (graphics) is provided (UEFI or BIOS VBE), draw a 200x100 rect.
    if let Some(fb) = boot_info.framebuffer.as_mut() {
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
                    // Simple color write (BGRX/RGBX-ish). Good enough for a demo.
                    buf[i] = 0xFF;               // Blue
                    if bpp > 1 { buf[i + 1] = 0x80; } // Green
                    if bpp > 2 { buf[i + 2] = 0x00; } // Red
                    if bpp > 3 { buf[i + 3] = 0x00; } // Alpha/unused
                }
            }
        }
    }

    loop { hlt(); }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { hlt(); }
}