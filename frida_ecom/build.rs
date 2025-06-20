use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=frida_ecom/config");

    // Get the output directory from cargo
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    // Navigate up to the target directory
    let target_dir = out_path
        .ancestors()
        .find(|p| p.ends_with("target"))
        .expect("Could not find target directory");

    // Create config directory in target
    let target_config_dir = target_dir.join("frida_ecom/config");
    fs::create_dir_all(&target_config_dir).expect("Failed to create config directory");

    // Copy config files
    fs::copy(
        "config/processor.toml",
        target_config_dir.join("processor.toml"),
    )
    .expect("Failed to copy processor config");

    fs::copy(
        "config/importer.toml",
        target_config_dir.join("importer.toml"),
    )
    .expect("Failed to copy importer config");
}
