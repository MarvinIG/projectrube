use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;

fn main() {
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
                        title: "Bevy Backend Probe".into(),
                        resolution: (800., 600.).into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_systems(Startup, (spawn_cam, print_backend))
        .run();
}

fn spawn_cam(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Camera::default(),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Visibility::default(),
    ));
}

fn print_backend(info: Res<RenderAdapterInfo>) {
    println!("Backend: {:?} | Adapter: {}", info.backend, info.name);
}
