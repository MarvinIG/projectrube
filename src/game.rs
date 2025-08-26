use bevy::prelude::*;

use crate::player::PlayerCam;
use crate::state::AppState;

/// Sets up the camera and lighting for the gameplay scene.
///
/// World and chunk generation are handled by the `WorldPlugin`.
pub fn setup_game(mut commands: Commands) {
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
}

pub fn return_to_menu(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::KeyP) {
        next_state.set(AppState::Menu);
    }
}

pub fn game_cleanup(
    mut commands: Commands,
    cams: Query<Entity, With<PlayerCam>>,
    lights: Query<Entity, With<DirectionalLight>>,
) {
    for e in &cams {
        commands.entity(e).despawn();
    }
    for e in &lights {
        commands.entity(e).despawn();
    }
}
