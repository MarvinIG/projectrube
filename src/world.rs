use bevy::prelude::*;

#[derive(Resource)]
pub struct WorldParams {
    pub width: i32,
    pub depth: i32,
}

impl Default for WorldParams {
    fn default() -> Self {
        Self {
            width: 512,
            depth: 512,
        }
    }
}
