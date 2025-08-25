use bevy::app::AppExit;
use bevy::input::mouse::MouseMotion;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::render::mesh::Mesh3d;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use fastnoise_lite::{FastNoiseLite, NoiseType};

// === Application State ===

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    Menu,
    Playing,
}

// === Resources ===

#[derive(Resource)]
struct WorldParams {
    width: i32,
    depth: i32,
}

impl Default for WorldParams {
    fn default() -> Self {
        Self { width: 10, depth: 10 }
    }
}

// === Components ===

#[derive(Component)]
struct PlayerCam {
    yaw: f32,
    pitch: f32,
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct DimText {
    dim: Dimension,
}

#[derive(Component)]
struct DimButton {
    dim: Dimension,
    delta: i32,
}

#[derive(Component)]
struct StartButton;

#[derive(Component)]
struct ExitButton;

#[derive(Clone, Copy)]
enum Dimension {
    Width,
    Depth,
}

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
                        title: "Voxel World".into(),
                        resolution: (800., 600.).into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .init_resource::<WorldParams>()
        .init_state::<AppState>()
        // Menu systems
        .add_systems(OnEnter(AppState::Menu), menu_setup)
        .add_systems(Update, menu_actions.run_if(in_state(AppState::Menu)))
        .add_systems(Update, update_dim_texts.run_if(in_state(AppState::Menu)))
        .add_systems(OnExit(AppState::Menu), menu_cleanup)
        // Game systems
        .add_systems(OnEnter(AppState::Playing), setup_game)
        .add_systems(Update, (mouse_look, keyboard_move).run_if(in_state(AppState::Playing)))
        .add_systems(Startup, print_backend)
        .run();
}

// === Menu ===

fn menu_setup(mut commands: Commands, params: Res<WorldParams>) {
    let root = commands
        .spawn((
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            MenuRoot,
        ))
        .id();

    // Title
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("Project Rube"),
            TextFont {
                font_size: 40.0,
                ..Default::default()
            },
        ));

        spawn_dim_row(parent, Dimension::Width, params.width);
        spawn_dim_row(parent, Dimension::Depth, params.depth);

        // Start button
        parent
            .spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                    margin: UiRect::all(Val::Px(5.0)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                StartButton,
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("Start Game"),
                    TextFont { font_size: 24.0, ..Default::default() },
                    TextColor::default(),
                ));
            });

        // Exit button
        parent
            .spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                    margin: UiRect::all(Val::Px(5.0)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                ExitButton,
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("Exit"),
                    TextFont { font_size: 24.0, ..Default::default() },
                    TextColor::default(),
                ));
            });
    });
}

fn spawn_dim_row(parent: &mut ChildSpawnerCommands, dim: Dimension, value: i32) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(5.0)),
                ..Default::default()
            },
        ))
        .with_children(|row| {
            let label = match dim {
                Dimension::Width => "Width:",
                Dimension::Depth => "Depth:",
            };
            row.spawn((
                Text::new(format!("{} {}", label, value)),
                TextFont { font_size: 24.0, ..Default::default() },
                TextColor::default(),
                DimText { dim },
            ));

            // minus button
            row
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                        margin: UiRect::left(Val::Px(5.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    DimButton { dim, delta: -1 },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("-"),
                        TextFont { font_size: 24.0, ..Default::default() },
                        TextColor::default(),
                    ));
                });

            // plus button
            row
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                        margin: UiRect::left(Val::Px(5.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    DimButton { dim, delta: 1 },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("+"),
                        TextFont { font_size: 24.0, ..Default::default() },
                        TextColor::default(),
                    ));
                });
        });
}

fn menu_actions(
    mut interaction_q: Query<(&Interaction, Option<&DimButton>, Option<&StartButton>, Option<&ExitButton>), Changed<Interaction>>,
    mut params: ResMut<WorldParams>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, dim_button, start, exit_button) in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if let Some(dim_button) = dim_button {
            let value = match dim_button.dim {
                Dimension::Width => &mut params.width,
                Dimension::Depth => &mut params.depth,
            };
            *value = (*value + dim_button.delta).max(1);
        }

        if start.is_some() {
            next_state.set(AppState::Playing);
        }

        if exit_button.is_some() {
            exit.write(AppExit::Success);
        }
    }
}

fn update_dim_texts(params: Res<WorldParams>, mut q: Query<(&DimText, &mut Text)>) {
    if !params.is_changed() {
        return;
    }
    for (dim, mut text) in &mut q {
        let label = match dim.dim {
            Dimension::Width => "Width:",
            Dimension::Depth => "Depth:",
        };
        *text = Text::new(format!("{} {}", label, match dim.dim {
            Dimension::Width => params.width,
            Dimension::Depth => params.depth,
        }));
    }
}

fn menu_cleanup(mut commands: Commands, q: Query<Entity, With<MenuRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// === Game Setup ===

fn setup_game(
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

    // generate terrain using Perlin noise
    let mut noise = FastNoiseLite::with_seed(0);
    noise.set_noise_type(Some(NoiseType::Perlin));

    let cube = meshes.add(Cuboid::default());
    let material = materials.add(Color::srgb_u8(150, 150, 150));
    for x in 0..params.width {
        for z in 0..params.depth {
            let n = noise.get_noise_2d(x as f32 * 0.1, z as f32 * 0.1);
            let height = (n * 3.0).round() as i32 + 1;
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

// === Player Controls ===

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
    if let Ok((mut transform, mut cam)) = q.single_mut() {
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
            let speed = 5.0;
            transform.translation += direction.normalize() * speed * time.delta_secs();
        }
    }
}

fn print_backend(info: Res<RenderAdapterInfo>) {
    println!("Backend: {:?} | Adapter: {}", info.backend, info.name);
}

