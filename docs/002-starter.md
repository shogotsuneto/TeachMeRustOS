# Tiny Rust OS — Starter

This guide shows a **minimal setup** to boot a tiny Rust kernel using the **modern `bootloader` ≥ 0.11** workflow.  
You’ll build a two-crate Cargo workspace:

- **`kernel/`** — a `#![no_std]` kernel using **`bootloader_api`** (UEFI/BIOS friendly).
- **`runner/`** — a tiny host tool that uses the **`bootloader`** crate to build bootable **BIOS**/**UEFI** disk images from your kernel and run QEMU.

> Why this setup? It avoids the legacy `bootimage` tool and works cleanly with current Rust/nightly and QEMU.

There is more comprehensive explanation for bare metal setup: [Writing an OS in Rust Philipp Oppermann's blog](https://os.phil-opp.com/freestanding-rust-binary/).
I believe ChatGPT must have referred to the blog.

## 1) Prerequisites (once per machine)

```bash
# Use nightly toolchain (pinned via rust-toolchain.toml below)
rustup toolchain install nightly
rustup component add llvm-tools-preview

# Add freestanding target for the kernel
rustup target add x86_64-unknown-none

# QEMU (virtual machine)
# Ubuntu/Debian:   sudo apt-get install qemu-system-x86
# Fedora:          sudo dnf install qemu-system-x86-core
# Arch:            sudo pacman -S qemu-full
# macOS (brew):    brew install qemu

# Optional (for UEFI graphics): install OVMF/edk2-ovmf package
# Ubuntu/Debian:   sudo apt-get install ovmf
```

---

## 2) Project layout

```
my-os/
├─ Cargo.toml                  # workspace
├─ rust-toolchain.toml         # pins nightly + tools
├─ .cargo/config.toml          # enables artifact deps (bindeps) for the workspace
├─ kernel/
│  ├─ Cargo.toml
│  ├─ .cargo/config.toml       # kernel-only: build-std for freestanding target
│  └─ src/
│     └─ main.rs               # minimal kernel (framebuffer rectangle demo)
└─ runner/
   ├─ Cargo.toml
   ├─ build.rs                 # builds BIOS + UEFI disk images
   └─ src/main.rs              # runs QEMU (UEFI if OVMF found; else BIOS)
```

---

## 3) files

Copy these files into the **exact paths** shown above. (If a file already exists, merge or replace its contents.)

### 3.1 Workspace `Cargo.toml` (root)

```toml
[workspace]
members = ["kernel", "runner"]
resolver = "2"
```

### 3.2 Root `.cargo/config.toml` (enables artifact dependencies)

```toml
[unstable]
bindeps = true
```

### 3.3 Root `rust-toolchain.toml` (pin nightly)

```toml
[toolchain]
channel = "nightly"
components = ["llvm-tools-preview"]
```

### 3.4 Kernel `.cargo/config.toml` (kernel-only build-std)

```toml
[build]
target = "x86_64-unknown-none"

[unstable]
build-std = ["core", "compiler_builtins", "alloc"]
build-std-features = ["compiler-builtins-mem"]
```

### 3.5 `kernel/Cargo.toml`

```toml
[package]
name = "kernel"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "kernel"
path = "src/main.rs"

[dependencies]
bootloader_api = "0.11.11"
x86_64 = "0.15"
spin = "0.9"

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

### 3.6 `kernel/src/main.rs` (draw a small rectangle if a framebuffer is present)

```rust
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
```

> Tip: Want text logs in your terminal? Add a simple **COM1 serial** writer and run QEMU with `-serial stdio`. (You can add this later.)

### 3.7 `runner/Cargo.toml` (artifact dependency on the kernel + bootloader)

```toml
[package]
name = "runner"
version = "0.1.0"
edition = "2021"

[build-dependencies]
bootloader = "0.11.11"
kernel = { path = "../kernel", artifact = "bin", target = "x86_64-unknown-none" }

[dependencies]
```

### 3.8 `runner/build.rs` (create BIOS + UEFI images)

```rust
use std::{env, path::PathBuf};

fn main() {
    // Path to the compiled kernel binary (from the artifact dependency)
    let kernel_bin = PathBuf::from(env::var_os("CARGO_BIN_FILE_KERNEL_kernel").expect("kernel artifact not found"));

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let uefi_img = out_dir.join("uefi.img");
    let bios_img = out_dir.join("bios.img");

    // Build UEFI and BIOS disk images
    let mut uefi = bootloader::UefiBoot::new(&kernel_bin);
    uefi.create_disk_image(&uefi_img).expect("create UEFI image");

    let mut bios = bootloader::BiosBoot::new(&kernel_bin);
    bios.create_disk_image(&bios_img).expect("create BIOS image");

    // Export paths for runner/src/main.rs
    println!("cargo:rustc-env=UEFI_IMAGE={}", uefi_img.display());
    println!("cargo:rustc-env=BIOS_IMAGE={}", bios_img.display());
}
```

### 3.9 `runner/src/main.rs` (run QEMU)

```rust
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
```

---

## 4) Build & Run

From the workspace root (where the top-level `Cargo.toml` lives):

```bash
# make sure nightly is picked up
cargo -V     # should show "nightly"

# build + run (creates disk images and launches QEMU)
cargo run -p runner
```

- If **OVMF** is installed (or you set `OVMF_PATH=/path/to/OVMF_CODE.fd`), the runner uses **UEFI** and you’ll see a colored rectangle.
- Without OVMF, it falls back to **BIOS** (still framebuffer in most cases), and you should still see a rectangle.
- **Headless** mode (useful on servers):
  ```bash
  QEMU_HEADLESS=1 cargo run -p runner
  ```
  This routes serial I/O to your terminal and disables the display window with `-nographic`.

---

## 5) Troubleshooting

**Cargo says**: `artifact = … requires -Z bindeps`  
→ Ensure **root** `.cargo/config.toml` has:

```toml
[unstable]
bindeps = true
```

…and you’re on nightly (`cargo -V` shows “nightly”). If needed, try:

```bash
cargo +nightly run -Z bindeps -p runner
```

**QEMU window closes immediately**  
→ Make sure the runner adds `-no-reboot -no-shutdown`. The example above does.  
Also confirm your QEMU install: `qemu-system-x86_64 --version`.

**I have OVMF installed but UEFI isn’t used**  
→ Set the OVMF path explicitly when running:

```bash
OVMF_PATH=/usr/share/OVMF/OVMF_CODE.fd cargo run -p runner
```

(Your distro’s path may differ.)

**No visible text output from kernel**  
→ The modern boot path uses a **graphics framebuffer** by default, so the old VGA text memory (0xb8000) often isn’t shown. For terminal logs, add a simple **serial (COM1)** writer and run with `-serial stdio` (already in the runner).

---

## 6) Next steps

- Add a **serial logger** (COM1) so your kernel can print text to the terminal.
- Set up a **page table** and explore higher-half memory.
- Implement a **heap allocator** (bring in `alloc`) and try `Vec`, `String`, etc.
- Handle **interrupts** and map the **keyboard**.
- Build a tiny **task executor** and a basic **timer**.

Happy hacking! 🦀
