use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::render::mesh::Mesh3d;
use fastnoise_lite::{FastNoiseLite, NoiseType};

use crate::player::PlayerCam;
use crate::world::WorldParams;

pub fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    params: Res<WorldParams>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        PlayerCam { yaw: 0.0, pitch: 0.0 },
        Visibility::default(),
    ));

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mut noise = FastNoiseLite::with_seed(0);
    noise.set_noise_type(Some(NoiseType::Perlin));
    noise.set_frequency(Some(0.1));

    let cube = meshes.add(Cuboid::default());
    let material = materials.add(Color::srgb_u8(150, 150, 150));

    for x in 0..params.width {
        for z in 0..params.depth {
            let n = noise.get_noise_2d(x as f32, z as f32);
            let height = (n * 3.0).round() as i32 + 1;
            if height > 0 {
                for y in 0..height {
                    commands.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_xyz(x as f32, y as f32 - 1.0, z as f32),
                    ));
                }
            }
        }
    }
}
