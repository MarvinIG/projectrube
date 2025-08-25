use bevy::app::AppExit;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use crate::state::AppState;
use crate::world::WorldParams;

#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct MenuCamera;

#[derive(Component)]
pub struct DimText {
    pub dim: Dimension,
}

#[derive(Component)]
pub struct DimButton {
    pub dim: Dimension,
    pub delta: i32,
}

#[derive(Component)]
pub struct StartButton;

#[derive(Component)]
pub struct ExitButton;

#[derive(Clone, Copy)]
pub enum Dimension {
    Width,
    Depth,
}

pub fn menu_setup(mut commands: Commands, params: Res<WorldParams>) {
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

        spawn_dim_row(parent, Dimension::Width, params.width);
        spawn_dim_row(parent, Dimension::Depth, params.depth);

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

pub fn menu_actions(
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

pub fn update_dim_texts(params: Res<WorldParams>, mut q: Query<(&DimText, &mut Text)>) {
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
