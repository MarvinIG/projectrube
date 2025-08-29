use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, Mesh3d};
use bevy::render::primitives::{Aabb, Frustum};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use block_mesh::ndshape::{ConstShape3u32, Shape};
use block_mesh::{
    GreedyQuadsBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG, Voxel, VoxelVisibility, greedy_quads,
};
use fastnoise_lite::{FastNoiseLite, NoiseType};
use futures_lite::future;

use crate::player::PlayerCam;
use crate::settings::NoiseSettings;
use crate::state::AppState;

/// Size of one cubic chunk edge in blocks.
pub const CHUNK_SIZE: i32 = 32;
/// Maximum vertical height of the world in blocks.
pub const MAX_HEIGHT: i32 = 256;
/// Number of vertical chunks making up the world height.
pub const MAX_CHUNKS_Y: i32 = MAX_HEIGHT / CHUNK_SIZE;

const CHUNK_SIZE_U32: u32 = CHUNK_SIZE as u32;
const LOD2_SIZE_U32: u32 = CHUNK_SIZE_U32 / 2;

/// Runtime-configurable world generation parameters.
#[derive(Resource)]
pub struct WorldParams {
    /// Number of chunks to generate outwards from the player along each axis.
    pub view_width: i32,
}

impl Default for WorldParams {
    fn default() -> Self {
        Self { view_width: 24 }
    }
}

/// Handle to the material used for all chunks.
#[derive(Resource)]
struct ChunkMaterial(pub Handle<StandardMaterial>);

/// Mapping of generated chunk coordinates to entities.
#[derive(Resource, Default)]
struct ChunkMap {
    entities: HashMap<IVec3, Entity>,
}

/// Pending background generation tasks.
///
/// Each entry tracks the requested level of detail so that
/// pending work can be cancelled or replaced if the player
/// approaches a chunk and it needs to be regenerated at a
/// higher resolution.
#[derive(Resource, Default)]
struct PendingTasks {
    tasks: HashMap<IVec3, (u32, Task<(IVec3, u32, Mesh)>)>,
}

/// Component tagging a chunk mesh entity.
#[derive(Component)]
pub struct Chunk {
    pub coord: IVec3,
    pub lod: u32,
}

/// Plugin managing world chunk generation and rendering.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMap>()
            .init_resource::<PendingTasks>()
            .add_systems(OnEnter(AppState::Playing), setup_chunk_material)
            .add_systems(
                Update,
                (
                    spawn_required_chunks,
                    process_chunk_tasks,
                    frustum_cull_chunks,
                )
                    .run_if(in_state(AppState::Playing)),
            )
            .add_systems(OnExit(AppState::Playing), cleanup_chunks);
    }
}

fn setup_chunk_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });
    commands.insert_resource(ChunkMaterial(material));
}

fn spawn_required_chunks(
    mut commands: Commands,
    params: Res<WorldParams>,
    settings: Res<NoiseSettings>,
    mut pending: ResMut<PendingTasks>,
    mut map: ResMut<ChunkMap>,
    player: Query<&Transform, With<PlayerCam>>,
    chunks: Query<&Chunk>,
) {
    let pool = AsyncComputeTaskPool::get();
    let player_pos = player.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);
    let player_chunk = IVec3::new(
        (player_pos.x / CHUNK_SIZE as f32).floor() as i32,
        (player_pos.y / CHUNK_SIZE as f32).floor() as i32,
        (player_pos.z / CHUNK_SIZE as f32).floor() as i32,
    );

    // Despawn chunks far outside the view radius
    let mut to_remove = Vec::new();
    for (coord, entity) in map.entities.iter() {
        let dist = (coord.x - player_chunk.x)
            .abs()
            .max((coord.z - player_chunk.z).abs());
        if dist > params.view_width + 2 {
            commands.entity(*entity).despawn();
            to_remove.push(*coord);
        }
    }
    for coord in to_remove {
        map.entities.remove(&coord);
    }

    // Queue missing chunks for generation
    for x in -params.view_width..=params.view_width {
        for z in -params.view_width..=params.view_width {
            let dist = x.abs().max(z.abs());
            let required_lod = if dist <= 3 { 1 } else { 2 };
            for y in 0..MAX_CHUNKS_Y {
                let coord = IVec3::new(player_chunk.x + x, y, player_chunk.z + z);

            if let Some(&entity) = map.entities.get(&coord) {
                if let Ok(chunk) = chunks.get(entity) {
                    if chunk.lod != required_lod {
                        commands.entity(entity).despawn();
                        map.entities.remove(&coord);
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if let Some((lod, _)) = pending.tasks.get(&coord) {
                if *lod == required_lod {
                    continue;
                }
                pending.tasks.remove(&coord);
            }

                let settings = settings.clone();
                let task = pool.spawn(async move {
                    let mesh = generate_chunk_mesh(coord, required_lod, settings);
                    (coord, required_lod, mesh)
                });
                pending.tasks.insert(coord, (required_lod, task));
            }
        }
    }
}

fn process_chunk_tasks(
    mut commands: Commands,
    mut pending: ResMut<PendingTasks>,
    mut map: ResMut<ChunkMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<ChunkMaterial>,
) {
    let mut finished = Vec::new();
    for (coord, (_lod, task)) in pending.tasks.iter_mut() {
        if let Some((c, lod, mesh)) = future::block_on(future::poll_once(task)) {
            let handle = meshes.add(mesh);
            let entity = commands
                .spawn((
                    Mesh3d(handle),
                    MeshMaterial3d(material.0.clone()),
                    Transform::from_xyz(
                        c.x as f32 * CHUNK_SIZE as f32,
                        c.y as f32 * CHUNK_SIZE as f32,
                        c.z as f32 * CHUNK_SIZE as f32,
                    ),
                    Visibility::default(),
                    Chunk { coord: c, lod },
                ))
                .id();
            map.entities.insert(c, entity);
            finished.push(*coord);
        }
    }
    for coord in finished {
        pending.tasks.remove(&coord);
    }
}

fn cleanup_chunks(
    mut commands: Commands,
    chunks: Query<Entity, With<Chunk>>,
    mut map: ResMut<ChunkMap>,
    mut pending: ResMut<PendingTasks>,
) {
    for e in &chunks {
        commands.entity(e).despawn();
    }
    map.entities.clear();
    pending.tasks.clear();
}

fn frustum_cull_chunks(
    cam: Query<(&Frustum, &GlobalTransform), With<Camera3d>>,
    mut q: Query<(&Transform, &mut Visibility), With<Chunk>>,
) {
    let Ok((frustum, _cam_transform)) = cam.single() else {
        return;
    };
    let aabb = Aabb::from_min_max(Vec3::ZERO, Vec3::splat(CHUNK_SIZE as f32));
    for (transform, mut vis) in &mut q {
        let world_from_local = Affine3A::from_mat4(transform.compute_matrix());
        let visible = frustum.intersects_obb(&aabb, &world_from_local, true, true);
        *vis = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

// === Meshing ===

#[derive(Clone, Copy, Eq, PartialEq)]
enum BlockType {
    Empty,
    Grass,
    Dirt,
    Stone,
}

const EMPTY: BlockType = BlockType::Empty;
const GRASS: BlockType = BlockType::Grass;
const DIRT: BlockType = BlockType::Dirt;
const STONE: BlockType = BlockType::Stone;

impl Voxel for BlockType {
    fn get_visibility(&self) -> VoxelVisibility {
        match self {
            BlockType::Empty => VoxelVisibility::Empty,
            _ => VoxelVisibility::Opaque,
        }
    }
}

impl MergeVoxel for BlockType {
    type MergeValue = BlockType;
    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}

fn generate_chunk_mesh(coord: IVec3, lod: u32, settings: NoiseSettings) -> Mesh {
    match lod {
        1 => build_mesh::<{ CHUNK_SIZE_U32 + 3 }>(coord, lod, &settings),
        2 => build_mesh::<{ LOD2_SIZE_U32 + 3 }>(coord, lod, &settings),
        _ => build_mesh::<{ CHUNK_SIZE_U32 + 3 }>(coord, 1, &settings),
    }
}

fn build_mesh<const N: u32>(coord: IVec3, lod: u32, settings: &NoiseSettings) -> Mesh {
    let size = N - 2;

    let shape = ConstShape3u32::<{ N }, { N }, { N }> {};
    let mut voxels = vec![EMPTY; (N * N * N) as usize];

    // 2D terrain noise layers for varied heights
    let mut noises = Vec::new();
    for layer in &settings.layers {
        let mut n = FastNoiseLite::with_seed(layer.seed);
        n.set_noise_type(Some(NoiseType::Perlin));
        n.set_frequency(Some(layer.frequency));
        noises.push((n, layer.amplitude));
    }

    // 3D noise for sparse caves and cliffs
    let mut cave = FastNoiseLite::with_seed(3);
    cave.set_noise_type(Some(NoiseType::Perlin));
    cave.set_frequency(Some(0.05));

    for z in 0..=size + 1 {
        for x in 0..=size + 1 {
            let wx = coord.x * CHUNK_SIZE + ((x as i32 - 1) * lod as i32);
            let wz = coord.z * CHUNK_SIZE + ((z as i32 - 1) * lod as i32);

            let mut height = 20.0;
            for (noise, amp) in &noises {
                height += noise.get_noise_2d(wx as f32, wz as f32) * amp;
            }
            let height = height.clamp(1.0, (MAX_HEIGHT - 1) as f32).round() as i32;

            for y in 1..=size + 1 {
                let wy = coord.y * CHUNK_SIZE + ((y as i32 - 1) * lod as i32);
                if wy > height {
                    continue;
                }

                let noise = cave.get_noise_3d(wx as f32, wy as f32, wz as f32);
                if noise > 0.9 {
                    continue; // carve cave
                }
                let idx = shape.linearize([x, y, z]) as usize;
                voxels[idx] = if wy == height {
                    GRASS
                } else if wy == height - 1 {
                    DIRT
                } else {
                    STONE
                };
            }
        }
    }

    let mut buffer = GreedyQuadsBuffer::new(voxels.len());
    greedy_quads(
        &voxels,
        &shape,
        [1; 3],
        [size + 1; 3],
        &RIGHT_HANDED_Y_UP_CONFIG.faces,
        &mut buffer,
    );

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for (face, group) in RIGHT_HANDED_Y_UP_CONFIG
        .faces
        .iter()
        .zip(buffer.quads.groups.iter())
    {
        for quad in group.iter() {
            let start = positions.len() as u32;
            let mut face_positions = face.quad_mesh_positions(quad, lod as f32);
            for p in &mut face_positions {
                p[0] -= lod as f32;
                p[1] -= lod as f32;
                p[2] -= lod as f32;
            }
            positions.extend_from_slice(&face_positions);
            normals.extend_from_slice(&face.quad_mesh_normals());
            indices.extend_from_slice(&face.quad_mesh_indices(start));

            let voxel = voxels[shape.linearize(quad.minimum) as usize];
            let color = match voxel {
                GRASS => [0.1, 0.8, 0.1, 1.0],
                DIRT => [0.55, 0.27, 0.07, 1.0],
                STONE => [0.6, 0.6, 0.6, 1.0],
                _ => [1.0, 1.0, 1.0, 1.0],
            };
            colors.extend_from_slice(&[color; 4]);
        }
    }

    use bevy::render::mesh::PrimitiveTopology;
    use bevy::render::render_asset::RenderAssetUsages;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
