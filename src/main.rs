mod state;
mod world;
mod menu;
mod game;
mod player;

use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;

use game::setup_game;
use menu::{menu_actions, menu_cleanup, menu_setup, update_dim_texts};
use player::{keyboard_move, mouse_look};
use state::AppState;
use world::WorldParams;

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
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::Menu), menu_setup)
        .add_systems(Update, menu_actions.run_if(in_state(AppState::Menu)))
        .add_systems(Update, update_dim_texts.run_if(in_state(AppState::Menu)))
        .add_systems(OnExit(AppState::Menu), menu_cleanup)
        .add_systems(OnEnter(AppState::Playing), setup_game)
        .add_systems(Update, (mouse_look, keyboard_move).run_if(in_state(AppState::Playing)))
        .add_systems(Startup, print_backend)
        .run();
}

fn print_backend(info: Res<RenderAdapterInfo>) {
    println!("Backend: {:?} | Adapter: {}", info.backend, info.name);
}
