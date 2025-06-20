use common::yaml_include::load_yaml_with_includes;
use std::{error::Error, fs, io::Write, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let project_name = "ecom";
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed={}/config", project_name);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    let env = std::env::var("FRIDA_ENV").unwrap_or_else(|_| "dev".to_string());

    // Navigate up to the target directory
    let target_dir = out_path
        .ancestors()
        .find(|p| p.ends_with("target"))
        .expect("Could not find target directory")
        .join(std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string()));

    // Create config directory in target
    let total_config_dir = target_dir.join("config");
    let total_config_file_name = total_config_dir.join("total_config.yaml");
    let source_config_dir = target_dir.join(&format!("../../{}/config/", project_name));

    println!("cargo:warning=Loading config env {:?} profile {:?}", env, std::env::var("PROFILE"));
    let path = source_config_dir.join(&format!("{}.yaml", env));
    let config_yaml = load_yaml_with_includes(&path)?;

    let mut out_str = String::new();
    {
        let mut emitter = yaml_rust2::YamlEmitter::new(&mut out_str);
        emitter.dump(&config_yaml)?
    }

    println!(
        "cargo:warning=Writing config to {:?}",
        total_config_file_name
    );

    fs::create_dir_all(total_config_dir)?;
    fs::File::create(total_config_file_name)?.write_all(out_str.as_bytes())?;

    Ok(())
}
