// Debug graphics initialization
use particle_sim::*;
use std::panic;
use std::process;

fn main() {
    println!("=== Graphics Debug Test ===");

    // Set panic hook to catch any graphics panics
    panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC in graphics: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!(
                "  at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        if let Some(msg) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("  message: {}", msg);
        } else if let Some(msg) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("  message: {}", msg);
        }
        process::exit(1);
    }));

    println!("Creating quarkstrom config...");
    let config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(800, 600),
    };
    println!("Config created successfully");

    println!("Calling quarkstrom::run...");

    // This is where it probably crashes
    let result = std::panic::catch_unwind(|| {
        quarkstrom::run::<renderer::Renderer>(config);
    });

    match result {
        Ok(_) => println!("Quarkstrom run completed successfully"),
        Err(_) => {
            eprintln!("Quarkstrom crashed!");
            process::exit(1);
        }
    }
}
