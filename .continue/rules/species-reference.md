# Species Reference

## Species Enum

All particle types are defined in `src/body/types.rs` as the `Species` enum. Properties are in `src/species.rs`.

## Electrolyte Species

| Species           | Mass (amu) | Radius (Å) | Charge | Description                   |
|-------------------|------------|------------|--------|-------------------------------|
| `LithiumIon`      | 6.94       | 0.76       | +1     | Li+ cation in electrolyte     |
| `ElectrolyteAnion`| 145.0      | 2.0        | -1     | PF6- counter-ion              |

## Metal Species

| Species       | Mass (amu) | Radius (Å) | LJ Enabled | Description                    |
|---------------|------------|------------|------------|--------------------------------|
| `LithiumMetal`| 6.94       | 1.52       | Yes        | Li0 deposited/electrode metal  |
| `FoilMetal`   | 1.0e6      | 1.52       | Yes        | Current collector (stationary) |

FoilMetal has extremely high mass to remain stationary during simulation.

## Solvent Molecules

| Species | Mass (amu) | Radius (Å) | Polar Charge | Description                    |
|---------|------------|------------|--------------|--------------------------------|
| `EC`    | 88.0       | 2.2        | 0.40         | Ethylene carbonate             |
| `DMC`   | 90.0       | 2.5        | 0.11         | Dimethyl carbonate             |
| `VC`    | 86.0       | 2.1        | 0.42         | Vinylene carbonate (additive)  |
| `FEC`   | 106.0      | 2.2        | 0.45         | Fluoroethylene carbonate       |
| `EMC`   | 104.0      | 2.4        | 0.11         | Ethyl methyl carbonate         |

## Solid Electrolytes

| Species | Mass (amu) | Radius (Å) | Description                           |
|---------|------------|------------|---------------------------------------|
| `LLZO`  | 200.0      | 2.5        | Li7La3Zr2O12 garnet                   |
| `LLZT`  | 200.0      | 2.5        | Li7La3Zr2O12 with Ta doping           |
| `S40B`  | 180.0      | 2.4        | Li2S-P2S5 sulfide glass electrolyte   |

## Electrode Materials (Intercalation)

### Anodes
| Species        | Mass (amu) | Radius (Å) | Potential (V) | Description              |
|----------------|------------|------------|---------------|--------------------------|
| `Graphite`     | 12.0       | 1.8        | ~0.1          | Graphitic carbon         |
| `HardCarbon`   | 12.0       | 1.8        | ~0.1          | Non-graphitizable carbon |
| `SiliconOxide` | 60.0       | 2.0        | ~0.3          | SiOx composite           |
| `LTO`          | 150.0      | 2.2        | ~1.55         | Li4Ti5O12 "zero-strain"  |

### Cathodes
| Species | Mass (amu) | Radius (Å) | Potential (V) | Description               |
|---------|------------|------------|---------------|---------------------------|
| `LFP`   | 158.0      | 2.0        | ~3.4          | LiFePO4 olivine           |
| `LMFP`  | 160.0      | 2.0        | ~3.5          | LiMn0.6Fe0.4PO4           |
| `NMC`   | 97.0       | 2.0        | ~3.7          | LiNixMnyCozO2 layered     |
| `NCA`   | 97.0       | 2.0        | ~3.7          | LiNiCoAlO2 layered        |

## SEI (Solid Electrolyte Interphase)

| Species | Mass (amu) | Radius (Å) | Description                              |
|---------|------------|------------|------------------------------------------|
| `SEI`   | 100.0      | 2.0        | Decomposition products (immobile layer)  |

SEI forms when solvent molecules are reduced at low potentials. Once formed, SEI particles are immobile and passivate the electrode surface.

## Adding New Species

1. Add variant to `Species` enum in `src/body/types.rs`
2. Add properties to `SPECIES_PROPERTIES` in `src/species.rs`
3. Update `update_species()` in `src/body/types.rs` if auto-conversion needed
4. Add neutral electron count in `src/config.rs`
5. Update electron hopping logic in `src/simulation/electron_hopping.rs` if needed
