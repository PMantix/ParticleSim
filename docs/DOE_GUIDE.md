# Design of Experiments (DOE) System

## Overview

The DOE system provides automated Design of Experiments capability for systematic parametric studies of the ParticleSim electrochemical system. It enables:

- **Headless execution**: Run simulations without GUI interaction
- **Automatic measurements**: Sample lithium metal deposition at specified locations
- **Parallel execution**: Distribute cases across multiple workstations
- **Excel-ready output**: Export time-series data as CSV files

## Quick Start

### 1. Generate DOE Configuration

```bash
cargo run --release --bin doe_runner generate switch_charging_study.toml
```

This creates a full factorial DOE with:
- **Overpotentials**: 0.7, 0.8, 0.9
- **Switching frequencies**: 500, 750, 1000, 1250, 1500 steps
- **Total cases**: 18 (3 conventional + 15 switch-charging)

### 2. List All Test Cases

```bash
cargo run --release --bin doe_runner list switch_charging_study.toml
```

### 3. Run a Specific Case

```bash
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.7_FREQ1000
```

###4. Run All Cases

```bash
cargo run --release --bin doe_runner run-all switch_charging_study.toml
```

## Distributing Work Across Workstations

To run cases in parallel across multiple machines:

1. **Copy the project** to each workstation
2. **Generate the same DOE config** on all machines (or copy the `.toml` file)
3. **Assign cases** to each machine:

**Workstation 1:**
```bash
cargo run --release --bin doe_runner run switch_charging_study.toml CONV_OP0.7
cargo run --release --bin doe_runner run switch_charging_study.toml CONV_OP0.8
cargo run --release --bin doe_runner run switch_charging_study.toml CONV_OP0.9
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.7_FREQ500
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.7_FREQ750
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.7_FREQ1000
```

**Workstation 2:**
```bash
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.7_FREQ1250
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.7_FREQ1500
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.8_FREQ500
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.8_FREQ750
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.8_FREQ1000
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.8_FREQ1250
```

**Workstation 3:**
```bash
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.8_FREQ1500
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.9_FREQ500
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.9_FREQ750
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.9_FREQ1000
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.9_FREQ1250
cargo run --release --bin doe_runner run switch_charging_study.toml SWITCH_OP0.9_FREQ1500
```

4. **Collect results**: Each machine saves CSVs to `doe_results/Switch_Charging_Study/`
5. **Consolidate**: Copy all CSV files to one location for analysis

## Configuration File Format

The DOE configuration is stored in TOML format:

```toml
study_name = "Switch Charging Study"
base_scenario = "default"
run_duration_fs = 70000.0
measurement_interval_fs = 1000.0

[[measurements]]
x = 0.0
y = 0.0
direction = "left"
width_ang = 70.0
label = "Position_3"

[[test_cases]]
case_id = "SWITCH_OP0.7_FREQ1000"
mode = "SwitchCharging"
overpotential = 1.4
switching_frequency_steps = 1000
group_a_foils = [1, 3, 5]
group_b_foils = [2, 4]
```

### Configuration Parameters

- **study_name**: Descriptive name for the DOE study
- **base_scenario**: Name of saved scenario to load (or "default")
- **run_duration_fs**: Simulation duration in femtoseconds (e.g., 70000)
- **measurement_interval_fs**: How often to sample measurements (e.g., 1000 = every 1ps)

### Measurement Points

Each measurement point defines:
- **x, y**: Coordinates for measurement location
- **direction**: "left", "right", "up", or "down" (measurement direction)
- **width_ang**: Width of measurement region in angstroms
- **label**: Name for this measurement position

### Test Cases

Each test case specifies:
- **case_id**: Unique identifier (used for filename)
- **mode**: "Conventional" or "SwitchCharging"
- **overpotential**: Setpoint value (doubled for switch-charging)
- **switching_frequency_steps**: Steps per cycle (switch-charging only)
- **group_a_foils**: Foil IDs for group A (e.g., [1, 3, 5])
- **group_b_foils**: Foil IDs for group B (e.g., [2, 4])

## Output Files

### Individual Case CSVs

Location: `doe_results/Switch_Charging_Study/<case_id>.csv`

Format:
```csv
Time_fs,Position_1_Edge,Position_2_Edge,Position_3_Edge,Position_4_Edge,Position_5_Edge,Position_1_LiMetal,Position_2_LiMetal,Position_3_LiMetal,Position_4_LiMetal,Position_5_LiMetal,Position_1_LiIon,Position_2_LiIon,Position_3_LiIon,Position_4_LiIon,Position_5_LiIon
1000.0,-42.3,-43.1,-44.2,-43.5,-42.8,15,18,22,19,16,120,118,115,117,119
2000.0,-43.1,-43.8,-45.1,-44.2,-43.5,18,21,25,22,19,118,116,113,115,117
3000.0,-44.2,-45.1,-46.3,-45.5,-44.8,22,25,29,26,23,115,113,110,112,114
...
```

**Column Structure:**
- **Time_fs**: Simulation time in femtoseconds
- **Position_X_Edge**: Leading edge position of lithium metal deposition at measurement point X
  - Negative values = growth towards left (away from foil)
  - Tracks the furthest extent of Li metal plating
- **Position_X_LiMetal**: Number of lithium metal particles within the 70Å measurement region at position X
  - Indicates local deposition density
- **Position_X_LiIon**: Number of lithium ion particles within the 70Å measurement region at position X
  - Indicates local ion concentration near the deposition front

### DOE Summary

Location: `doe_results/Switch_Charging_Study/DOE_Summary.csv`

Format:
```csv
Case_ID,Mode,Overpotential,Switching_Freq,Final_Li_Metal_Count,Avg_Edge_Position,Max_Edge_Position
CONV_OP0.7,Conventional,0.7,N/A,150,-45.2,48.3
SWITCH_OP0.7_FREQ1000,SwitchCharging,1.4,1000,145,-43.1,46.7
...
```

## Customizing DOE Studies

### Custom Overpotentials and Frequencies

Edit the generated `.toml` file or create your own programmatically:

```rust
use particle_sim::doe::DoeConfig;

let config = DoeConfig::generate_switch_charging_doe(
    "Custom Study".to_string(),
    "my_scenario".to_string(),
    vec![0.6, 0.7, 0.8, 0.9, 1.0],  // Overpotentials
    vec![250, 500, 1000, 2000],      // Frequencies
    100000.0,                         // Duration (fs)
    500.0,                            // Measurement interval (fs)
);

config.to_file("custom_study.toml").unwrap();
```

### Custom Measurement Points

Modify the `[[measurements]]` sections in the TOML file to change:
- Number of measurement locations
- Positions (x, y coordinates)
- Measurement directions
- Region widths

### Custom Foil Assignments

Change `group_a_foils` and `group_b_foils` in each test case to study different electrode configurations.

## Best Practices

1. **Start small**: Test with 1-2 cases first to verify setup
2. **Use saved scenarios**: Create a well-equilibrated base scenario before running DOE
3. **Monitor progress**: Check output files during long runs
4. **Backup results**: Copy CSV files regularly during multi-day studies
5. **Document changes**: Keep notes on any manual configuration adjustments

## Troubleshooting

**"Scenario not found" warning:**
- Create a saved scenario via the GUI (State tab > Save As > "default")
- Or modify `base_scenario` in the TOML to match an existing saved state

**No lithium metal detected:**
- Increase `run_duration_fs` to allow more deposition time
- Check that foils are properly configured in the base scenario
- Verify measurement positions are near electrode surfaces

**Out of memory:**
- Reduce `run_duration_fs` or increase `measurement_interval_fs`
- Run fewer cases in parallel
- Close other applications

## Future Enhancements

Potential improvements to the DOE system:
- Response surface methodology (RSM) optimization
- Latin hypercube sampling for efficient design space exploration
- Automated statistical analysis (ANOVA, regression)
- Real-time progress visualization
- Checkpoint/resume capability for long runs
