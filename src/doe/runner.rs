use super::config::{ChargingMode, DoeConfig, TestCase};
use super::export::{export_doe_summary, export_results_to_csv};
use super::measurement::AutoMeasurement;
/// DOE runner for executing test cases headlessly
use crate::simulation::Simulation;
use crate::switch_charging::{Mode as ScMode, StepSetpoint};

pub struct DoeRunner {
    config: DoeConfig,
    output_dir: String,
}

impl DoeRunner {
    pub fn new(config: DoeConfig, output_dir: String) -> Self {
        Self { config, output_dir }
    }

    /// Run a specific test case by ID
    pub fn run_case(&self, case_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let case = self
            .config
            .test_cases
            .iter()
            .find(|c| c.case_id == case_id)
            .ok_or_else(|| format!("Case ID '{}' not found", case_id))?;

        println!("\n╔══════════════════════════════════════════╗");
        println!("║  Running DOE Case: {}  ", case_id);
        println!("╚══════════════════════════════════════════╝\n");

        self.execute_case(case)?;
        Ok(())
    }

    /// Run all test cases sequentially
    pub fn run_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut all_samples = Vec::new();

        for case in &self.config.test_cases {
            println!("\n╔══════════════════════════════════════════╗");
            println!("║  Running DOE Case: {}  ", case.case_id);
            println!("╚══════════════════════════════════════════╝\n");

            let samples = self.execute_case(case)?;
            all_samples.push(samples);
        }

        // Export summary
        export_doe_summary(&self.config.test_cases, &all_samples, &self.output_dir)?;

        println!(
            "\n✅ DOE study '{}' completed successfully!",
            self.config.study_name
        );
        println!("📊 Results saved to: {}", self.output_dir);

        Ok(())
    }

    /// Execute a single test case
    fn execute_case(
        &self,
        case: &TestCase,
    ) -> Result<Vec<super::measurement::MeasurementSample>, Box<dyn std::error::Error>> {
        // Create simulation
        let mut sim = Simulation::new();

        // Load base scenario
        self.load_scenario(&mut sim, &self.config.base_scenario)?;

        // Configure foil groups
        self.configure_foil_groups(&mut sim, case);

        // Configure charging mode
        match case.mode {
            ChargingMode::Conventional => {
                self.configure_conventional_charging(&mut sim, case);
            }
            ChargingMode::SwitchCharging => {
                self.configure_switch_charging(&mut sim, case);
            }
        }

        // Setup automatic measurements
        let mut auto_measurement = AutoMeasurement::new(
            self.config.measurements.clone(),
            self.config.measurement_interval_fs,
        );

        // Run simulation
        println!("⚙️  Mode: {:?}", case.mode);
        println!("⚙️  Overpotential: {}", case.overpotential);
        if let Some(freq) = case.switching_frequency_steps {
            println!("⚙️  Switching Frequency: {} steps", freq);
        }
        println!("⚙️  Duration: {} fs", self.config.run_duration_fs);
        println!(
            "⚙️  Measurement Interval: {} fs\n",
            self.config.measurement_interval_fs
        );

        let start_time = std::time::Instant::now();
        let mut current_time_fs = 0.0;
        let mut frame_count = 0;

        while current_time_fs < self.config.run_duration_fs {
            sim.step();
            current_time_fs += sim.dt;
            frame_count += 1;

            // Perform measurements
            auto_measurement.measure(&sim.bodies, current_time_fs);

            // Progress update every 10000 fs
            if frame_count % 10000 == 0 {
                let progress = (current_time_fs / self.config.run_duration_fs * 100.0) as u32;
                println!("  Progress: {}% ({:.0} fs)", progress, current_time_fs);
            }
        }

        let elapsed = start_time.elapsed();
        println!("✓ Simulation completed in {:.2}s", elapsed.as_secs_f32());

        // Export results
        let samples = auto_measurement.get_samples().to_vec();
        export_results_to_csv(case, &samples, &self.output_dir)?;

        Ok(samples)
    }

    /// Load scenario from saved state or configuration
    fn load_scenario(
        &self,
        sim: &mut Simulation,
        scenario_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Try loading from saved_state directory with multiple extensions
        let extensions = ["bin.gz", "json", "bin"];
        let mut loaded = false;

        for ext in &extensions {
            let state_path = format!("saved_state/{}.{}", scenario_name, ext);
            if std::path::Path::new(&state_path).exists() {
                let scenario = crate::io::load_state(&state_path)?;
                sim.load_state(scenario);
                println!("✓ Loaded scenario from: {}", state_path);
                loaded = true;
                break;
            }
        }

        if !loaded {
            println!(
                "⚠️  Scenario '{}' not found in saved_state/, using empty simulation",
                scenario_name
            );
        }
        Ok(())
    }

    /// Configure foil group assignments
    fn configure_foil_groups(&self, sim: &mut Simulation, case: &TestCase) {
        sim.group_a = case.group_a_foils.iter().copied().collect();
        sim.group_b = case.group_b_foils.iter().copied().collect();

        println!("✓ Configured Group A: {:?}", case.group_a_foils);
        println!("✓ Configured Group B: {:?}", case.group_b_foils);
    }

    /// Configure conventional charging (all foils active)
    fn configure_conventional_charging(&self, sim: &mut Simulation, case: &TestCase) {
        // Set all foils to the same overpotential setpoint
        for foil in &mut sim.foils {
            if let Some(ctrl) = foil.overpotential_controller.as_mut() {
                ctrl.target_ratio = case.overpotential;
            }
        }

        // Disable switch-charging
        sim.switch_run_state = crate::switch_charging::RunState::Idle;

        println!(
            "✓ Configured conventional charging at overpotential {}",
            case.overpotential
        );
    }

    /// Configure switch-charging mode
    fn configure_switch_charging(&self, sim: &mut Simulation, case: &TestCase) {
        let freq = case.switching_frequency_steps.unwrap_or(1000);

        // Enable global active/inactive mode
        sim.switch_config.use_global_active_inactive = true;
        sim.switch_config.global_active = StepSetpoint {
            mode: ScMode::Overpotential,
            value: case.overpotential as f64,
        };
        sim.switch_config.global_inactive = StepSetpoint {
            mode: ScMode::Overpotential,
            value: (2.0 - case.overpotential) as f64, // Complementary
        };

        // Set delta_steps for frequency
        sim.switch_config.delta_steps = freq as u32;

        // Assign foil roles
        sim.switch_config.role_to_foil.insert(
            crate::switch_charging::Role::PosA,
            case.group_a_foils.clone(),
        );
        sim.switch_config.role_to_foil.insert(
            crate::switch_charging::Role::NegA,
            case.group_b_foils.clone(),
        );

        // Start switch-charging
        sim.switch_run_state = crate::switch_charging::RunState::Running;
        sim.switch_scheduler.start(&sim.switch_config);

        println!("✓ Configured switch-charging:");
        println!("  - Frequency: {} steps", freq);
        println!("  - Active overpotential: {}", case.overpotential);
        println!("  - Inactive overpotential: {}", 2.0 - case.overpotential);
    }

    /// List all available test cases
    pub fn list_cases(&self) {
        println!("\n╔══════════════════════════════════════════╗");
        println!("║  DOE Study: {}  ", self.config.study_name);
        println!("╚══════════════════════════════════════════╝\n");

        println!("Total cases: {}\n", self.config.test_cases.len());

        for (idx, case) in self.config.test_cases.iter().enumerate() {
            println!("  [{}] {}", idx + 1, case.case_id);
            println!("      Mode: {:?}", case.mode);
            println!("      Overpotential: {}", case.overpotential);
            if let Some(freq) = case.switching_frequency_steps {
                println!("      Switching Freq: {} steps", freq);
            }
            println!();
        }
    }
}
