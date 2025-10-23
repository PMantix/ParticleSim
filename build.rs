use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../quarkstrom/quarkstrom");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Check if we need to update quarkstrom dependency
    let quarkstrom_path = "../quarkstrom/quarkstrom";

    if Path::new(quarkstrom_path).exists() {
        println!(
            "cargo:warning=Quarkstrom dependency found at {}",
            quarkstrom_path
        );

        // Optionally clean and rebuild quarkstrom if requested
        if env::var("FRESH_DEPS").is_ok() {
            println!("cargo:warning=FRESH_DEPS set - rebuilding quarkstrom dependency");

            let _output = Command::new("cargo")
                .args(&["clean"])
                .current_dir(quarkstrom_path)
                .output()
                .expect("Failed to clean quarkstrom");

            let _output = Command::new("cargo")
                .args(&["build"])
                .current_dir(quarkstrom_path)
                .output()
                .expect("Failed to build quarkstrom");
        }
    } else {
        println!(
            "cargo:warning=Quarkstrom dependency not found at expected path: {}",
            quarkstrom_path
        );
    }
}
