use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct NoiseLayer {
    pub seed: i32,
    pub frequency: f32,
    pub amplitude: f32,
}

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct NoiseSettings {
    pub layers: [NoiseLayer; 5],
}

impl Default for NoiseSettings {
    fn default() -> Self {
        if let Ok(data) = fs::read_to_string("settings.json") {
            if let Ok(cfg) = serde_json::from_str::<NoiseSettings>(&data) {
                return cfg;
            }
        }
        NoiseSettings {
            layers: [
                NoiseLayer {
                    seed: 0,
                    frequency: 0.01,
                    amplitude: 10.0,
                },
                NoiseLayer {
                    seed: 1,
                    frequency: 0.03,
                    amplitude: 5.0,
                },
                NoiseLayer {
                    seed: 2,
                    frequency: 0.08,
                    amplitude: 2.0,
                },
                NoiseLayer {
                    seed: 4,
                    frequency: 0.16,
                    amplitude: 1.0,
                },
                NoiseLayer {
                    seed: 5,
                    frequency: 0.32,
                    amplitude: 0.5,
                },
            ],
        }
    }
}

impl NoiseSettings {
    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write("settings.json", json);
        }
    }
}
