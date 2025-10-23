/// DOE configuration structures
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoeConfig {
    /// Name of the DOE study
    pub study_name: String,

    /// Base scenario/configuration file to load
    pub base_scenario: String,

    /// Duration to run each test case (in femtoseconds)
    pub run_duration_fs: f32,

    /// Measurement sampling interval (in femtoseconds)
    pub measurement_interval_fs: f32,

    /// Measurement configuration
    pub measurements: Vec<MeasurementPoint>,

    /// List of test cases to execute
    pub test_cases: Vec<TestCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementPoint {
    /// X coordinate for measurement (typically 0 for foil 3 centerline)
    pub x: f32,

    /// Y coordinate for measurement start
    pub y: f32,

    /// Measurement direction: "left", "right", "up", "down"
    pub direction: String,

    /// Width of measurement region (angstroms)
    pub width_ang: f32,

    /// Label for this measurement point
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Unique case ID
    pub case_id: String,

    /// Charging mode: "Conventional" or "SwitchCharging"
    pub mode: ChargingMode,

    /// Overpotential setpoint (active foils)
    pub overpotential: f32,

    /// For switch-charging: switching frequency in steps
    pub switching_frequency_steps: Option<u64>,

    /// Foil group A assignment (e.g., [1, 3, 5])
    pub group_a_foils: Vec<u64>,

    /// Foil group B assignment (e.g., [2, 4])
    pub group_b_foils: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChargingMode {
    /// Conventional charging: all foils active simultaneously
    Conventional,
    /// Switch-charging: alternating between groups
    SwitchCharging,
}

#[cfg(feature = "doe")]
impl DoeConfig {
    /// Generate a full factorial DOE for switch-charging study
    pub fn generate_switch_charging_doe(
        study_name: String,
        base_scenario: String,
        overpotentials: Vec<f32>,
        switching_frequencies: Vec<u64>,
        run_duration_fs: f32,
        measurement_interval_fs: f32,
    ) -> Self {
        let mut test_cases = Vec::new();

        // Default foil assignments
        let group_a = vec![1, 3, 5];
        let group_b = vec![2, 4];

        // Generate conventional charging cases (baseline)
        for &overpot in &overpotentials {
            test_cases.push(TestCase {
                case_id: format!("CONV_OP{:.1}", overpot),
                mode: ChargingMode::Conventional,
                overpotential: overpot,
                switching_frequency_steps: None,
                group_a_foils: group_a.clone(),
                group_b_foils: group_b.clone(),
            });
        }

        // Generate switch-charging cases
        for &overpot in &overpotentials {
            for &freq in &switching_frequencies {
                test_cases.push(TestCase {
                    case_id: format!("SWITCH_OP{:.1}_FREQ{}", overpot, freq),
                    mode: ChargingMode::SwitchCharging,
                    overpotential: overpot * 2.0, // Double current for switch-charging
                    switching_frequency_steps: Some(freq),
                    group_a_foils: group_a.clone(),
                    group_b_foils: group_b.clone(),
                });
            }
        }

        // Define measurement points (5 positions along y-axis on foil 3 centerline)
        let measurements = vec![
            MeasurementPoint {
                x: 0.0,
                y: -40.0,
                direction: "left".to_string(),
                width_ang: 70.0,
                label: "Position_1".to_string(),
            },
            MeasurementPoint {
                x: 0.0,
                y: -20.0,
                direction: "left".to_string(),
                width_ang: 70.0,
                label: "Position_2".to_string(),
            },
            MeasurementPoint {
                x: 0.0,
                y: 0.0,
                direction: "left".to_string(),
                width_ang: 70.0,
                label: "Position_3".to_string(),
            },
            MeasurementPoint {
                x: 0.0,
                y: 20.0,
                direction: "left".to_string(),
                width_ang: 70.0,
                label: "Position_4".to_string(),
            },
            MeasurementPoint {
                x: 0.0,
                y: 40.0,
                direction: "left".to_string(),
                width_ang: 70.0,
                label: "Position_5".to_string(),
            },
        ];

        DoeConfig {
            study_name,
            base_scenario,
            run_duration_fs,
            measurement_interval_fs,
            measurements,
            test_cases,
        }
    }

    /// Load DOE configuration from TOML file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save DOE configuration to TOML file
    pub fn to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}
