use bevy::input::mouse::MouseMotion;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::render::mesh::Mesh3d;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;

#[derive(Component)]
struct PlayerCam {
    yaw: f32,
    pitch: f32,
}

fn main() {
    println!("Starting program");
    // Try DX12 first; if it still fails, change to Backends::VULKAN and re-run.
    let forced = WgpuSettings {
        backends: Some(Backends::DX12), // or Backends::VULKAN
        ..Default::default()
    };

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(forced),
                    ..Default::default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Voxel World".into(),
                        resolution: (800., 600.).into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_systems(Startup, (setup, print_backend))
        .add_systems(Update, (mouse_look, keyboard_move))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Running Setup");
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

    // simple voxel world
    let cube = meshes.add(Cuboid::default());
    let material = materials.add(Color::srgb_u8(150, 150, 150));
    for x in -5..=5 {
        for z in -5..=5 {
            let height = if (x + z) % 2 == 0 { 1 } else { 2 };
            for y in 0..height {
                commands.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(x as f32, y as f32 - 1.0, z as f32),
                ));
            }
        }
    }
    println!("Finished Setup");
}

fn mouse_look(
    mut mouse_events: EventReader<MouseMotion>,
    mut q: Query<(&mut Transform, &mut PlayerCam)>,
) {
    let mut delta = Vec2::ZERO;
    for ev in mouse_events.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    if let Ok((mut transform, mut cam)) = q.get_single_mut() {
        let sensitivity = 0.002;
        cam.yaw -= delta.x * sensitivity;
        cam.pitch -= delta.y * sensitivity;
        cam.pitch = cam.pitch.clamp(-1.54, 1.54);
        transform.rotation =
            Quat::from_axis_angle(Vec3::Y, cam.yaw) * Quat::from_axis_angle(Vec3::X, cam.pitch);
    }
}

fn keyboard_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Transform, With<PlayerCam>>,
) {
    if let Ok(mut transform) = q.get_single_mut() {
        let mut direction = Vec3::ZERO;
        let forward = transform.forward();
        let right = transform.right();
        if keys.pressed(KeyCode::KeyW) {
            direction += *forward;
        }
        if keys.pressed(KeyCode::KeyS) {
            direction -= *forward;
        }
        if keys.pressed(KeyCode::KeyA) {
            direction -= *right;
        }
        if keys.pressed(KeyCode::KeyD) {
            direction += *right;
        }
        if keys.pressed(KeyCode::Space) {
            direction += Vec3::Y;
        }
        if keys.pressed(KeyCode::ShiftLeft) {
            direction -= Vec3::Y;
        }
        if direction.length_squared() > 0.0 {
            let speed = 5.0;
            transform.translation += direction.normalize() * speed * time.delta_secs();
        }
    }
}

fn print_backend(info: Res<RenderAdapterInfo>) {
    println!("Backend: {:?} | Adapter: {}", info.backend, info.name);
}

