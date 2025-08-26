use bevy::app::AppExit;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use crate::settings::NoiseSettings;
use crate::state::AppState;
use crate::world::WorldParams;

#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct MenuCamera;

#[derive(Component)]
pub struct ViewText;

#[derive(Component)]
pub struct ViewButton {
    pub delta: i32,
}

#[derive(Component)]
pub struct StartButton;

#[derive(Component)]
pub struct ExitButton;

#[derive(Component, Clone, Copy)]
pub enum NoiseField {
    Amplitude,
    Frequency,
}

#[derive(Component)]
pub struct NoiseText {
    pub layer: usize,
    pub field: NoiseField,
}

#[derive(Component)]
pub struct NoiseButton {
    pub layer: usize,
    pub field: NoiseField,
    pub delta: f32,
}

pub fn menu_setup(mut commands: Commands, params: Res<WorldParams>, settings: Res<NoiseSettings>) {
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

    commands.spawn((Camera2d, MenuCamera));

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("Project Rube"),
            TextFont {
                font_size: 40.0,
                ..Default::default()
            },
        ));

        spawn_view_row(parent, params.view_width);
        spawn_noise_rows(parent, &settings);

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
                    TextFont {
                        font_size: 24.0,
                        ..Default::default()
                    },
                    TextColor::default(),
                ));
            });

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
                    TextFont {
                        font_size: 24.0,
                        ..Default::default()
                    },
                    TextColor::default(),
                ));
            });
    });
}

fn spawn_view_row(parent: &mut ChildSpawnerCommands, value: i32) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            margin: UiRect::all(Val::Px(5.0)),
            ..Default::default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("View Width: {}", value)),
                TextFont {
                    font_size: 24.0,
                    ..Default::default()
                },
                TextColor::default(),
                ViewText,
            ));

            row.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                    margin: UiRect::left(Val::Px(5.0)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                ViewButton { delta: -1 },
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("-"),
                    TextFont {
                        font_size: 24.0,
                        ..Default::default()
                    },
                    TextColor::default(),
                ));
            });

            row.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                    margin: UiRect::left(Val::Px(5.0)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                ViewButton { delta: 1 },
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("+"),
                    TextFont {
                        font_size: 24.0,
                        ..Default::default()
                    },
                    TextColor::default(),
                ));
            });
        });
}

fn spawn_noise_rows(parent: &mut ChildSpawnerCommands, settings: &NoiseSettings) {
    for (i, layer) in settings.layers.iter().enumerate() {
        // amplitude row
        parent
            .spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(5.0)),
                ..Default::default()
            },))
            .with_children(|row| {
                row.spawn((
                    Text::new(format!("Layer {} Amp: {:.2}", i + 1, layer.amplitude)),
                    TextFont {
                        font_size: 24.0,
                        ..Default::default()
                    },
                    TextColor::default(),
                    NoiseText {
                        layer: i,
                        field: NoiseField::Amplitude,
                    },
                ));

                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                        margin: UiRect::left(Val::Px(5.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    NoiseButton {
                        layer: i,
                        field: NoiseField::Amplitude,
                        delta: -1.0,
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("-"),
                        TextFont {
                            font_size: 24.0,
                            ..Default::default()
                        },
                        TextColor::default(),
                    ));
                });

                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                        margin: UiRect::left(Val::Px(5.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    NoiseButton {
                        layer: i,
                        field: NoiseField::Amplitude,
                        delta: 1.0,
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("+"),
                        TextFont {
                            font_size: 24.0,
                            ..Default::default()
                        },
                        TextColor::default(),
                    ));
                });
            });

        // frequency row
        parent
            .spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(5.0)),
                ..Default::default()
            },))
            .with_children(|row| {
                row.spawn((
                    Text::new(format!("Layer {} Freq: {:.2}", i + 1, layer.frequency)),
                    TextFont {
                        font_size: 24.0,
                        ..Default::default()
                    },
                    TextColor::default(),
                    NoiseText {
                        layer: i,
                        field: NoiseField::Frequency,
                    },
                ));

                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                        margin: UiRect::left(Val::Px(5.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    NoiseButton {
                        layer: i,
                        field: NoiseField::Frequency,
                        delta: -0.01,
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("-"),
                        TextFont {
                            font_size: 24.0,
                            ..Default::default()
                        },
                        TextColor::default(),
                    ));
                });

                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                        margin: UiRect::left(Val::Px(5.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    NoiseButton {
                        layer: i,
                        field: NoiseField::Frequency,
                        delta: 0.01,
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("+"),
                        TextFont {
                            font_size: 24.0,
                            ..Default::default()
                        },
                        TextColor::default(),
                    ));
                });
            });
    }
}

pub fn menu_actions(
    mut interaction_q: Query<
        (
            &Interaction,
            Option<&ViewButton>,
            Option<&StartButton>,
            Option<&ExitButton>,
        ),
        Changed<Interaction>,
    >,
    mut params: ResMut<WorldParams>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, view_button, start, exit_button) in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if let Some(view_button) = view_button {
            params.view_width = (params.view_width + view_button.delta).max(1);
        }

        if start.is_some() {
            next_state.set(AppState::Playing);
        }

        if exit_button.is_some() {
            exit.write(AppExit::Success);
        }
    }
}

pub fn noise_actions(
    mut interaction_q: Query<(&Interaction, &NoiseButton), Changed<Interaction>>,
    mut settings: ResMut<NoiseSettings>,
) {
    for (interaction, button) in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let layer = &mut settings.layers[button.layer];
        match button.field {
            NoiseField::Amplitude => {
                layer.amplitude = (layer.amplitude + button.delta).max(0.0);
            }
            NoiseField::Frequency => {
                layer.frequency = (layer.frequency + button.delta).max(0.0);
            }
        }
    }
}

pub fn update_noise_text(settings: Res<NoiseSettings>, mut q: Query<(&mut Text, &NoiseText)>) {
    if !settings.is_changed() {
        return;
    }
    for (mut text, info) in &mut q {
        let layer = &settings.layers[info.layer];
        *text = Text::new(match info.field {
            NoiseField::Amplitude => {
                format!("Layer {} Amp: {:.2}", info.layer + 1, layer.amplitude)
            }
            NoiseField::Frequency => {
                format!("Layer {} Freq: {:.2}", info.layer + 1, layer.frequency)
            }
        });
    }
}

pub fn save_settings_on_l(keys: Res<ButtonInput<KeyCode>>, settings: Res<NoiseSettings>) {
    if keys.just_pressed(KeyCode::KeyL) {
        settings.save();
    }
}

pub fn update_view_text(params: Res<WorldParams>, mut q: Query<&mut Text, With<ViewText>>) {
    if !params.is_changed() {
        return;
    }
    for mut text in &mut q {
        *text = Text::new(format!("View Width: {}", params.view_width));
    }
}

pub fn menu_cleanup(
    mut commands: Commands,
    roots: Query<Entity, With<MenuRoot>>,
    cams: Query<Entity, With<MenuCamera>>,
) {
    for e in &roots {
        commands.entity(e).despawn();
    }
    for e in &cams {
        commands.entity(e).despawn();
    }
}
