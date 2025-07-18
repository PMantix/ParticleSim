// init_config.rs
// Handles loading and parsing the initial particle configuration from init_config.toml

use serde::{Deserialize, Serialize};
use crate::body::Species;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct InitConfig {
    pub simulation: Option<SimulationConfig>,
    pub particles: ParticlesConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SimulationConfig {
    pub domain_bounds: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ParticlesConfig {
    #[serde(default)]
    pub circles: Vec<CircleConfig>,
    #[serde(default)]
    pub metal_rectangles: Vec<MetalRectangleConfig>,
    #[serde(default)]
    pub foil_rectangles: Vec<FoilRectangleConfig>,
    #[serde(default)]
    pub random: Vec<RandomConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CircleConfig {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub species: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetalRectangleConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub species: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FoilRectangleConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub current: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RandomConfig {
    pub count: usize,
    pub species: String,
    pub domain_width: f32,
    pub domain_height: f32,
}

impl InitConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: InitConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_default() -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_from_file("init_config.toml")
    }
}

impl CircleConfig {
    pub fn to_species(&self) -> Result<Species, String> {
        match self.species.as_str() {
            "LithiumMetal" => Ok(Species::LithiumMetal),
            "LithiumIon" => Ok(Species::LithiumIon),
            "ElectrolyteAnion" => Ok(Species::ElectrolyteAnion),
            "FoilMetal" => Ok(Species::FoilMetal),
            "EC" => Ok(Species::EC),
            "DMC" => Ok(Species::DMC),
            _ => Err(format!("Unknown species: {}", self.species)),
        }
    }
}

impl MetalRectangleConfig {
    pub fn to_species(&self) -> Result<Species, String> {
        match self.species.as_str() {
            "LithiumMetal" => Ok(Species::LithiumMetal),
            "LithiumIon" => Ok(Species::LithiumIon),
            "ElectrolyteAnion" => Ok(Species::ElectrolyteAnion),
            "FoilMetal" => Ok(Species::FoilMetal),
            "EC" => Ok(Species::EC),
            "DMC" => Ok(Species::DMC),
            _ => Err(format!("Unknown species: {}", self.species)),
        }
    }

    /// Convert center coordinates to origin (bottom-left) coordinates
    /// for use with SimCommand::AddRectangle
    pub fn to_origin_coords(&self) -> (f32, f32) {
        (
            self.x - self.width / 2.0,
            self.y - self.height / 2.0,
        )
    }
}

impl FoilRectangleConfig {
    /// Convert center coordinates to origin (bottom-left) coordinates
    /// for use with SimCommand::AddFoil
    pub fn to_origin_coords(&self) -> (f32, f32) {
        (
            self.x - self.width / 2.0,
            self.y - self.height / 2.0,
        )
    }
}

impl RandomConfig {
    pub fn to_species(&self) -> Result<Species, String> {
        match self.species.as_str() {
            "LithiumMetal" => Ok(Species::LithiumMetal),
            "LithiumIon" => Ok(Species::LithiumIon),
            "ElectrolyteAnion" => Ok(Species::ElectrolyteAnion),
            "FoilMetal" => Ok(Species::FoilMetal),
            "EC" => Ok(Species::EC),
            "DMC" => Ok(Species::DMC),
            _ => Err(format!("Unknown species: {}", self.species)),
        }
    }
}
