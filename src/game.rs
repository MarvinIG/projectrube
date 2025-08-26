use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::render::mesh::Mesh3d;
use fastnoise_lite::{FastNoiseLite, NoiseType};

use crate::player::PlayerCam;
use crate::world::{CHUNK_SIZE, MAX_HEIGHT, WorldParams};

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
        PlayerCam {
            yaw: 0.0,
            pitch: 0.0,
        },
        Visibility::default(),
    ));

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mut base_noise = FastNoiseLite::with_seed(0);
    base_noise.set_noise_type(Some(NoiseType::Perlin));
    base_noise.set_frequency(Some(0.005));

    let mut detail_noise = FastNoiseLite::with_seed(1);
    detail_noise.set_noise_type(Some(NoiseType::Perlin));
    detail_noise.set_frequency(Some(0.02));

    let mut cave_noise = FastNoiseLite::with_seed(2);
    cave_noise.set_noise_type(Some(NoiseType::Perlin));
    cave_noise.set_frequency(Some(0.08));

    let cube = meshes.add(Cuboid::default());
    let material = materials.add(Color::srgb_u8(150, 150, 150));

    for cx in -params.view_width..=params.view_width {
        for cz in -params.view_width..=params.view_width {
            let chunk_x = cx * CHUNK_SIZE;
            let chunk_z = cz * CHUNK_SIZE;
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let world_x = chunk_x + x;
                    let world_z = chunk_z + z;
                    let base = base_noise.get_noise_2d(world_x as f32, world_z as f32);
                    let detail = detail_noise.get_noise_2d(world_x as f32, world_z as f32);
                    let height = ((base * 20.0) + (detail * 5.0) + 20.0)
                        .round()
                        .clamp(1.0, (MAX_HEIGHT - 1) as f32)
                        as i32;
                    for y in 0..height {
                        if y == 0
                            || cave_noise.get_noise_3d(world_x as f32, y as f32, world_z as f32)
                                > 0.0
                        {
                            commands.spawn((
                                Mesh3d(cube.clone()),
                                MeshMaterial3d(material.clone()),
                                Transform::from_xyz(world_x as f32, y as f32, world_z as f32),
                            ));
                        }
                    }
                }
            }
        }
    }
}
