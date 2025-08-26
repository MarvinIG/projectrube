mod game;
mod menu;
mod player;
mod settings;
mod state;
mod world;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};

use game::{game_cleanup, return_to_menu, setup_game};
use menu::{
    menu_actions, menu_cleanup, menu_setup, noise_actions, save_settings_on_l, update_noise_text,
    update_view_text,
};
use player::{keyboard_move, mouse_look};
use settings::NoiseSettings;
use state::AppState;
use world::{WorldParams, WorldPlugin};

fn main() {
    let forced = WgpuSettings {
        backends: Some(Backends::DX12),
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
        .init_resource::<NoiseSettings>()
        .add_plugins(WorldPlugin)
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::Menu), menu_setup)
        .add_systems(Update, menu_actions.run_if(in_state(AppState::Menu)))
        .add_systems(Update, noise_actions.run_if(in_state(AppState::Menu)))
        .add_systems(Update, update_view_text.run_if(in_state(AppState::Menu)))
        .add_systems(Update, update_noise_text.run_if(in_state(AppState::Menu)))
        .add_systems(Update, save_settings_on_l.run_if(in_state(AppState::Menu)))
        .add_systems(OnExit(AppState::Menu), menu_cleanup)
        .add_systems(OnEnter(AppState::Playing), setup_game)
        .add_systems(
            Update,
            (mouse_look, keyboard_move, return_to_menu).run_if(in_state(AppState::Playing)),
        )
        .add_systems(OnExit(AppState::Playing), game_cleanup)
        .add_systems(Startup, print_backend)
        .run();
}

fn print_backend(info: Res<RenderAdapterInfo>) {
    println!("Backend: {:?} | Adapter: {}", info.backend, info.name);
}
