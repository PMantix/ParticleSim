/// CLI tool for running Design of Experiments (DOE) cases
use particle_sim::doe::{DoeConfig, DoeRunner};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return;
    }
    
    let command = &args[1];
    
    match command.as_str() {
        "generate" => generate_doe_config(&args[2..]),
        "list" => list_cases(&args[2..]),
        "run" => run_case(&args[2..]),
        "run-all" => run_all_cases(&args[2..]),
        _ => {
            println!("Unknown command: {}", command);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  ParticleSim DOE Runner - Design of Experiments Tool  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    println!("Usage: cargo run --release --bin doe_runner <command> [options]\n");
    println!("Commands:");
    println!("  generate    Generate DOE configuration file");
    println!("  list        List all test cases in a DOE configuration");
    println!("  run         Run a specific test case");
    println!("  run-all     Run all test cases sequentially\n");
    println!("Examples:");
    println!("  # Generate DOE config for switch-charging study");
    println!("  cargo run --release --bin doe_runner generate switch_charging_study.toml\n");
    println!("  # List all cases");
    println!("  cargo run --release --bin doe_runner list switch_charging_study.toml\n");
    println!("  # Run specific case");
    println!("  cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.7_FREQ1000\n");
    println!("  # Run all cases");
    println!("  cargo run --release --bin doe_runner run-all switch_charging_study.toml\n");
}

fn generate_doe_config(args: &[String]) {
    if args.is_empty() {
        println!("❌ Error: Please specify output file name");
        println!("Usage: cargo run --bin doe_runner generate <output_file.toml>");
        return;
    }
    
    let output_file = &args[0];
    
    println!("\n🔧 Generating DOE configuration...\n");
    
    // Generate switch-charging DOE with your specified parameters
    let overpotentials = vec![0.7, 0.8, 0.9];
    let switching_frequencies = vec![500, 750, 1000, 1250, 1500];
    
    let config = DoeConfig::generate_switch_charging_doe(
        "Switch Charging Study".to_string(),
        "default".to_string(), // Base scenario name
        overpotentials,
        switching_frequencies,
        70000.0, // Run duration in fs
        1000.0,  // Measurement interval in fs
    );
    
    match config.to_file(output_file) {
        Ok(_) => {
            println!("✅ DOE configuration generated: {}", output_file);
            println!("📊 Total test cases: {}", config.test_cases.len());
            println!("   - Conventional charging: 3 cases");
            println!("   - Switch-charging: {} cases\n", config.test_cases.len() - 3);
        }
        Err(e) => {
            println!("❌ Error generating config: {}", e);
        }
    }
}

fn list_cases(args: &[String]) {
    if args.is_empty() {
        println!("❌ Error: Please specify DOE configuration file");
        println!("Usage: cargo run --bin doe_runner list <config_file.toml>");
        return;
    }
    
    let config_file = &args[0];
    
    match DoeConfig::from_file(config_file) {
        Ok(config) => {
            let runner = DoeRunner::new(config, "doe_results".to_string());
            runner.list_cases();
        }
        Err(e) => {
            println!("❌ Error loading config: {}", e);
        }
    }
}

fn run_case(args: &[String]) {
    if args.len() < 2 {
        println!("❌ Error: Please specify config file and case ID");
        println!("Usage: cargo run --bin doe_runner run <config_file.toml> <case_id>");
        return;
    }
    
    let config_file = &args[0];
    let case_id = &args[1];
    
    match DoeConfig::from_file(config_file) {
        Ok(config) => {
            let output_dir = format!("doe_results/{}", config.study_name.replace(" ", "_"));
            let runner = DoeRunner::new(config, output_dir);
            
            match runner.run_case(case_id) {
                Ok(_) => println!("\n✅ Case '{}' completed successfully!\n", case_id),
                Err(e) => println!("❌ Error running case: {}\n", e),
            }
        }
        Err(e) => {
            println!("❌ Error loading config: {}", e);
        }
    }
}

fn run_all_cases(args: &[String]) {
    if args.is_empty() {
        println!("❌ Error: Please specify DOE configuration file");
        println!("Usage: cargo run --bin doe_runner run-all <config_file.toml>");
        return;
    }
    
    let config_file = &args[0];
    
    match DoeConfig::from_file(config_file) {
        Ok(config) => {
            let output_dir = format!("doe_results/{}", config.study_name.replace(" ", "_"));
            let runner = DoeRunner::new(config, output_dir);
            
            match runner.run_all() {
                Ok(_) => {},
                Err(e) => println!("❌ Error running DOE: {}\n", e),
            }
        }
        Err(e) => {
            println!("❌ Error loading config: {}", e);
        }
    }
}
