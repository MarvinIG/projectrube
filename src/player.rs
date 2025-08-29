use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

#[derive(Component)]
pub struct PlayerCam {
    pub yaw: f32,
    pub pitch: f32,
}

pub fn mouse_look(
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
    if let Ok((mut transform, mut cam)) = q.single_mut() {
        let sensitivity = 0.002;
        cam.yaw -= delta.x * sensitivity;
        cam.pitch -= delta.y * sensitivity;
        cam.pitch = cam.pitch.clamp(-1.54, 1.54);
        transform.rotation =
            Quat::from_axis_angle(Vec3::Y, cam.yaw) * Quat::from_axis_angle(Vec3::X, cam.pitch);
    }
}

pub fn keyboard_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Transform, With<PlayerCam>>,
) {
    if let Ok(mut transform) = q.single_mut() {
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
            let speed = 25.0;
            transform.translation += direction.normalize() * speed * time.delta_secs();
        }
    }
}
