# Plotting and Analysis Features

## Overview

The particle simulation now includes a comprehensive plotting and analysis system that allows you to visualize and analyze simulation results in real-time or at specific time points. The system is designed to be flexible, extensible, and user-friendly.

## Features

### 1. Plot Types

**Spatial Profiles**
- **Spatial Profile (X)**: Mean quantities plotted vs X position 
- **Spatial Profile (Y)**: Mean quantities plotted vs Y position
- Shows how properties vary spatially across the simulation domain

**Time Series**
- Plots quantities over time (e.g., species populations, current)
- Continuous real-time plotting with configurable time windows
- Ideal for tracking system evolution

**Concentration Map** (Planned)
- 2D heatmap showing species concentrations
- Visual representation of particle distributions

**Charge Distribution**
- Shows charge distribution across space
- Can be plotted vs position or tracked over time

**Species Population**
- Tracks the number of particles of each species over time
- Useful for monitoring reaction progress

**Current Analysis**
- Compares command current vs actual electron flow
- Helps analyze foil performance and electron transport

### 2. Quantities Available for Plotting

- **Charge**: Total charge of particles
- **Electron Count**: Number of electrons per particle
- **Velocity**: Particle velocities (x or y components)
- **Species Concentration**: Density of specific species
- **Total Species Count**: Number of particles of each type
- **Foil Current**: Current through specific foils
- **Electron Hop Rate**: Rate of electron transfer events
- **Local Field Strength**: Electric field strength at positions

### 3. Sampling Modes

**Single Timestep**
- Captures data at the current simulation time
- Good for snapshots and manual analysis
- Updates only when manually triggered

**Continuous**
- Real-time data collection during simulation
- Configurable update frequency (Hz)
- Automatic time window management

**Time Averaged** (Planned)
- Averages data over specified time windows
- Reduces noise in fluctuating quantities

### 4. GUI Controls

#### Main Controls Panel
Located in the main simulation window under "Data Analysis & Plotting":

- **Open Plotting Window**: Access the full plotting control panel
- **Quick Plot Buttons**: 
  - Li+ Population: Tracks lithium ion count over time
  - Charge vs X: Shows charge distribution along X-axis
  - Velocity Profile: Velocity distribution along X-axis  
  - Foil Current: Monitors foil current over time

#### Plotting Control Panel
Comprehensive interface for creating custom plots:

- **Plot Configuration**:
  - Title: Custom name for your plot
  - Plot Type: Select from available plot types
  - Quantity: Choose what to measure
  - Sampling Mode: How to collect data

- **Parameters**:
  - Spatial Bins: Resolution for spatial plots (10-200)
  - Time Window: How much history to keep (1-60 seconds)  
  - Update Frequency: How often to refresh (0.1-30 Hz)

#### Individual Plot Windows
Each plot opens in its own window with:
- Real-time data display
- Configuration summary
- Data statistics and range information
- Manual update controls for single-timestep plots
- Clear data functionality

### 5. Data Export

Each plot can export data in multiple formats:

- **CSV**: Comma-separated values for spreadsheet analysis
- **JSON**: Structured data format for programmatic use  
- **TSV**: Tab-separated values

Export files are automatically timestamped and saved to a `plots/` directory.

## Usage Examples

### Monitoring Species Evolution
1. Click "Li+ Population" quick plot button
2. Watch lithium ion count change during electrochemical reactions
3. Export data for further analysis

### Analyzing Charge Distribution  
1. Open Plotting Window → Create New Plot
2. Set Plot Type: "Spatial Profile (X)"
3. Set Quantity: "Charge"
4. Set Sampling: "Single Timestep"
5. Click "Create Plot" and manually update as needed

### Current-Voltage Analysis
1. Create "Current Analysis" plot for specific foil
2. Set to continuous sampling at 10 Hz
3. Compare command vs actual current in real-time
4. Export data for I-V curve analysis

### Concentration Mapping
1. Set up "Species Concentration" plots for each species
2. Use spatial profiles to see concentration gradients
3. Track how concentrations change over time

## Technical Implementation

### Architecture
- **PlottingSystem**: Core system managing all plot windows
- **PlotWindow**: Individual plot instance with data and configuration
- **PlotConfig**: Configuration settings for each plot type
- **PlotData**: Time-series data storage with metadata

### Data Flow
1. Simulation updates → Bodies and Foils data
2. PlottingSystem.update_plots() called each frame
3. Data collection based on sampling mode and frequency
4. GUI displays current data in plot windows
5. Export saves data with timestamp

### Performance Considerations
- Configurable update frequencies to balance responsiveness vs performance
- Automatic data windowing to limit memory usage
- Efficient spatial binning for large particle counts
- Background data collection doesn't block simulation

## Future Enhancements

### Planned Features
- **2D Concentration Maps**: Heat map visualization
- **Vector Field Plots**: Velocity and force field visualization  
- **Correlation Analysis**: Cross-correlation between quantities
- **Statistical Analysis**: Mean, std dev, distributions
- **Advanced Export**: Custom data ranges and formats

### Extensibility
The plotting system is designed to be easily extended:
- Add new plot types by extending the PlotType enum
- Add new quantities by extending the Quantity enum
- Custom analysis functions in the analysis module
- New export formats via the ExportFormat enum

## Tips and Best Practices

1. **Performance**: Use lower update frequencies (1-5 Hz) for computationally intensive plots
2. **Memory**: Set appropriate time windows to avoid excessive memory usage
3. **Analysis**: Use single-timestep mode for detailed analysis of specific moments
4. **Export**: Export data regularly during long simulations for backup
5. **Spatial Resolution**: Balance spatial bins vs performance - start with 50 bins
6. **Multiple Views**: Create multiple plots of the same quantity with different parameters

## Troubleshooting

**Plot not updating**: Check if sampling mode is set correctly and simulation is running
**Poor performance**: Reduce update frequency or spatial resolution
**No data visible**: Ensure particles exist and quantities are non-zero
**Export fails**: Check that plots/ directory is writable

The plotting system provides powerful analysis capabilities while maintaining the simulation's performance and usability. It's designed to grow with your analysis needs and support both real-time monitoring and detailed post-processing workflows.
