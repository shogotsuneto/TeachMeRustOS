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