use bevy::prelude::*;

/// Size of one cubic chunk edge in blocks.
pub const CHUNK_SIZE: i32 = 32;

/// Maximum vertical height of the world in blocks.
pub const MAX_HEIGHT: i32 = 128;

/// Runtime-configurable world generation parameters.
#[derive(Resource)]
pub struct WorldParams {
    /// Number of chunks to generate outwards from the origin along each axis.
    pub view_width: i32,
}

impl Default for WorldParams {
    fn default() -> Self {
        Self { view_width: 4 }
    }
}
