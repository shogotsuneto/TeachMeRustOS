use std::env;
use std::fs;
use std::process::{Command, Stdio};

fn qemu_exists(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn find_ovmf() -> Option<String> {
    if let Ok(p) = env::var("OVMF_PATH") {
        if fs::metadata(&p).is_ok() { return Some(p); }
    }
    let candidates = [
        "/usr/share/OVMF/OVMF_CODE.fd",                 // Debian/Ubuntu
        "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd",        // Arch
        "/usr/share/edk2/ovmf/OVMF_CODE.fd",            // Fedora
        "/opt/homebrew/Cellar/edk2/ovmf/OVMF_CODE.fd",  // macOS Homebrew (varies)
    ];
    for c in candidates {
        if fs::metadata(c).is_ok() { return Some(c.to_string()); }
    }
    None
}

fn main() {
    let bios_img = env!("BIOS_IMAGE");
    let uefi_img = env!("UEFI_IMAGE");

    let qemu = if qemu_exists("qemu-system-x86_64") {
        "qemu-system-x86_64"
    } else if qemu_exists("qemu-system-x86_64.exe") {
        "qemu-system-x86_64.exe"
    } else {
        eprintln!("qemu-system-x86_64 not found in PATH");
        std::process::exit(1);
    };

    let headless = env::var("QEMU_HEADLESS").is_ok();
    let display_args: [&str; 2] = if headless { ["-display", "curses"] } else { ["-vga", "std"] };

    if let Some(ovmf) = find_ovmf() {
        let mut cmd = Command::new(qemu);
        cmd.args([
            "-bios", &ovmf,
            "-drive", &format!("format=raw,file={}", uefi_img),
            "-m", "256M",
            "-machine", "q35",
            "-serial", "stdio",
            "-no-reboot",
            "-no-shutdown",
        ]);
        cmd.args(display_args);
        eprintln!("Running QEMU (UEFI): {:?}", cmd);
        let status = cmd.status().expect("failed to start qemu (uefi)");
        eprintln!("QEMU (UEFI) exited with: {status}");
        if status.success() { return; }
    }

    let mut cmd = Command::new(qemu);
    cmd.args([
        "-drive", &format!("format=raw,file={}", bios_img),
        "-m", "256M",
        "-machine", "pc-i440fx-6.2",
        "-boot", "order=c",
        "-serial", "stdio",
        "-no-reboot",
        "-no-shutdown",
        "-d", "guest_errors",
    ]);
    cmd.args(display_args);
    eprintln!("Running QEMU (BIOS): {:?}", cmd);
    let status = cmd.status().expect("failed to start qemu (bios)");
    eprintln!("QEMU (BIOS) exited with: {status}");
}
