# Electrolyte Solution Feature

## Overview
Added a new "Add Electrolyte (EC/DMC)" feature to the GUI scenario tab that allows users to add realistic electrolyte solutions with proper stoichiometry.

## Features

### Controls
- **Molarity Input**: Adjustable molarity of LiPF6 (0.1M to 5M typical range)
- **Total Particles**: Set the total number of particles to distribute (100-10000)
- **Add Button**: "Add Electrolyte (EC/DMC)" button to generate the solution
- **Composition Display**: Shows the calculated particle counts for each species

### Chemistry
The feature implements realistic electrolyte chemistry:
- **LiPF6 Dissociation**: Li+ + PF6- (1:1 stoichiometry)
- **Solvent Mixture**: EC:DMC in 1:1 volume ratio
- **Solvation**: ~15 solvent molecules per salt molecule (typical for battery electrolytes)

### Calculation Method
1. **Salt Fraction**: Based on molarity and typical solvation numbers
2. **Ion Distribution**: Equal Li+ and PF6- counts from LiPF6 dissociation
3. **Solvent Split**: Remaining particles divided equally between EC and DMC
4. **Random Placement**: All particles placed randomly throughout the domain

## Usage
1. Navigate to the Scenario tab in the GUI
2. Adjust the molarity (e.g., 1.0M for 1M LiPF6)
3. Set the desired total particle count (e.g., 1000)
4. Click "Add Electrolyte (EC/DMC)" to generate the solution
5. View the composition breakdown to verify the particle distribution

## Example
For 1M LiPF6 with 1000 total particles:
- Li+: ~59 particles
- PF6-: ~59 particles  
- EC: ~441 particles
- DMC: ~441 particles

This represents a realistic electrolyte solution for lithium-ion battery simulations.
