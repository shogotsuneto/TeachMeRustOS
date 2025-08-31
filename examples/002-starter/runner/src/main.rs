use std::env;
use std::process::{Command, Stdio};

fn main() {
    let bios_img = env!("BIOS_IMAGE");
    let uefi_img = env!("UEFI_IMAGE");

    // Prefer UEFI if OVMF is available (set OVMF_PATH if needed)
    let ovmf_path = env::var("OVMF_PATH").ok();
    let headless = env::var("QEMU_HEADLESS").is_ok();

    let mut cmd;
    if let Some(ovmf) = ovmf_path {
        cmd = Command::new("qemu-system-x86_64");
        cmd.args([
            "-bios", &ovmf,
            "-drive", &format!("format=raw,file={}", uefi_img),
            "-m", "256M",
            "-machine", "q35",
            "-serial", "stdio",
            "-no-reboot",
            "-no-shutdown",
        ]);
        if headless { cmd.arg("-nographic"); } else { cmd.args(&["-vga","std"]); }
    } else {
        cmd = Command::new("qemu-system-x86_64");
        cmd.args([
            "-drive", &format!("format=raw,file={}", bios_img),
            "-m", "256M",
            "-machine", "pc",
            "-boot", "order=c",
            "-serial", "stdio",
            "-no-reboot",
            "-no-shutdown",
        ]);
        if headless { cmd.arg("-nographic"); } else { cmd.args(&["-vga","std"]); }
    }

    let status = cmd.status().expect("failed to start qemu");
    eprintln!("QEMU exited with: {status}");
}