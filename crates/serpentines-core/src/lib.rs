//! Serpentines core engine: platform-agnostic logic for trails, particles, and presets.

use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub age: f32,
    pub lifetime: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailPreset {
    pub name: String,
    pub max_particles: u32,
    pub emission_rate: f32,
    pub decay_seconds: f32,
    pub color_start: Vec4,
    pub color_end: Vec4,
}

impl Default for TrailPreset {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            max_particles: 4096,
            emission_rate: 120.0,
            decay_seconds: 0.6,
            color_start: Vec4::new(1.0, 1.0, 1.0, 1.0),
            color_end: Vec4::new(1.0, 1.0, 1.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub preset: TrailPreset,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            preset: TrailPreset::default(),
        }
    }
}

/// Basic engine state placeholder. Later: particle pool, spline smoothing, etc.
pub struct TrailEngine {
    pub config: EngineConfig,
}

impl TrailEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    pub fn update(&mut self, _dt: f32) {
        // TODO: advance particles
    }
}
