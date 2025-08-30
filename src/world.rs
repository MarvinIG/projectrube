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
use std::sync::{Arc, Mutex};

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

#[derive(Resource, Default)]
struct LastChunk(pub Option<IVec3>);

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
            .init_resource::<LastChunk>()
            .add_systems(
                OnEnter(AppState::Playing),
                (
                    setup_chunk_material,
                    setup_noise_resources,
                    reset_player_chunk,
                ),
            )
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

#[derive(Resource, Clone)]
struct NoiseResources {
    layers: Vec<(Arc<Mutex<FastNoiseLite>>, f32)>,
    cave: Arc<Mutex<FastNoiseLite>>,
    cliff: Arc<Mutex<FastNoiseLite>>,
    boulder_density: Arc<Mutex<FastNoiseLite>>,
    boulder_scatter: Arc<Mutex<FastNoiseLite>>,
    boulder_shape: Arc<Mutex<FastNoiseLite>>,
    tree_density: Arc<Mutex<FastNoiseLite>>,
    tree_scatter: Arc<Mutex<FastNoiseLite>>,
}

impl NoiseResources {
    fn from_settings(settings: &NoiseSettings) -> Self {
        let mut layers = Vec::new();
        for layer in &settings.layers {
            let mut n = FastNoiseLite::with_seed(layer.seed);
            n.set_noise_type(Some(NoiseType::Perlin));
            n.set_frequency(Some(layer.frequency));
            layers.push((Arc::new(Mutex::new(n)), layer.amplitude));
        }

        let mut cave = FastNoiseLite::with_seed(3);
        cave.set_noise_type(Some(NoiseType::Perlin));
        cave.set_frequency(Some(0.05));

        let mut cliff = FastNoiseLite::with_seed(99);
        cliff.set_noise_type(Some(NoiseType::Perlin));
        cliff.set_frequency(Some(0.01));

        let mut boulder_density = FastNoiseLite::with_seed(1337);
        boulder_density.set_noise_type(Some(NoiseType::Perlin));
        boulder_density.set_frequency(Some(0.003));

        let mut boulder_scatter = FastNoiseLite::with_seed(1338);
        boulder_scatter.set_noise_type(Some(NoiseType::Perlin));
        boulder_scatter.set_frequency(Some(0.08));

        let mut boulder_shape = FastNoiseLite::with_seed(1339);
        boulder_shape.set_noise_type(Some(NoiseType::Perlin));
        boulder_shape.set_frequency(Some(0.3));

        let mut tree_density = FastNoiseLite::with_seed(4242);
        tree_density.set_noise_type(Some(NoiseType::Perlin));
        tree_density.set_frequency(Some(0.005));

        let mut tree_scatter = FastNoiseLite::with_seed(4243);
        tree_scatter.set_noise_type(Some(NoiseType::Perlin));
        tree_scatter.set_frequency(Some(0.1));

        Self {
            layers,
            cave: Arc::new(Mutex::new(cave)),
            cliff: Arc::new(Mutex::new(cliff)),
            boulder_density: Arc::new(Mutex::new(boulder_density)),
            boulder_scatter: Arc::new(Mutex::new(boulder_scatter)),
            boulder_shape: Arc::new(Mutex::new(boulder_shape)),
            tree_density: Arc::new(Mutex::new(tree_density)),
            tree_scatter: Arc::new(Mutex::new(tree_scatter)),
        }
    }
}

fn setup_noise_resources(mut commands: Commands, settings: Res<NoiseSettings>) {
    commands.insert_resource(NoiseResources::from_settings(&settings));
}

fn reset_player_chunk(mut last_chunk: ResMut<LastChunk>) {
    last_chunk.0 = None;
}

fn spawn_required_chunks(
    mut commands: Commands,
    params: Res<WorldParams>,
    noise: Res<NoiseResources>,
    mut pending: ResMut<PendingTasks>,
    mut map: ResMut<ChunkMap>,
    player: Query<&Transform, With<PlayerCam>>,
    chunks: Query<&Chunk>,
    mut last_chunk: ResMut<LastChunk>,
) {
    let pool = AsyncComputeTaskPool::get();
    let player_pos = player.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);
    let player_chunk = IVec3::new(
        (player_pos.x / CHUNK_SIZE as f32).floor() as i32,
        (player_pos.y / CHUNK_SIZE as f32).floor() as i32,
        (player_pos.z / CHUNK_SIZE as f32).floor() as i32,
    );

    if last_chunk.0.map_or(false, |c| c == player_chunk) {
        return;
    }

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
            let required_lod = if dist <= 6 { 1 } else { 2 };
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

                let noise = noise.clone();
                let task = pool.spawn(async move {
                    let mesh = generate_chunk_mesh(coord, required_lod, &noise);
                    (coord, required_lod, mesh)
                });
                pending.tasks.insert(coord, (required_lod, task));
            }
        }
    }

    last_chunk.0 = Some(player_chunk);
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
    Wood,
    Leaf,
}

const EMPTY: BlockType = BlockType::Empty;
const GRASS: BlockType = BlockType::Grass;
const DIRT: BlockType = BlockType::Dirt;
const STONE: BlockType = BlockType::Stone;
const WOOD: BlockType = BlockType::Wood;
const LEAF: BlockType = BlockType::Leaf;

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

fn generate_chunk_mesh(coord: IVec3, lod: u32, noise: &NoiseResources) -> Mesh {
    match lod {
        1 => build_mesh::<{ CHUNK_SIZE_U32 + 3 }>(coord, lod, noise),
        2 => build_mesh::<{ LOD2_SIZE_U32 + 3 }>(coord, lod, noise),
        _ => build_mesh::<{ CHUNK_SIZE_U32 + 3 }>(coord, 1, noise),
    }
}

fn build_mesh<const N: u32>(coord: IVec3, lod: u32, resources: &NoiseResources) -> Mesh {
    let size = N - 2;

    let shape = ConstShape3u32::<{ N }, { N }, { N }> {};
    let mut voxels = vec![EMPTY; (N * N * N) as usize];
    let size_i32 = size as i32;
    let occ_stride = size as i32;
    let mut occupancy = vec![i32::MIN; (size * size) as usize];

    let set_block = |voxels: &mut Vec<BlockType>,
                     wx: i32,
                     wy: i32,
                     wz: i32,
                     block: BlockType,
                     occupancy: &mut Vec<i32>| {
        let lx = ((wx - coord.x * CHUNK_SIZE) / lod as i32) + 1;
        let ly = ((wy - coord.y * CHUNK_SIZE) / lod as i32) + 1;
        let lz = ((wz - coord.z * CHUNK_SIZE) / lod as i32) + 1;
        if lx >= 0
            && lx <= size_i32 + 1
            && ly >= 0
            && ly <= size_i32 + 1
            && lz >= 0
            && lz <= size_i32 + 1
        {
            let idx = shape.linearize([lx as u32, ly as u32, lz as u32]) as usize;
            voxels[idx] = block;
            if block != EMPTY {
                let ox = lx - 1;
                let oz = lz - 1;
                if ox >= 0 && ox < occ_stride && oz >= 0 && oz < occ_stride {
                    let occ = (ox + oz * occ_stride) as usize;
                    if wy > occupancy[occ] {
                        occupancy[occ] = wy;
                    }
                }
            }
        }
    };

    let cave = &resources.cave;
    let cliff = &resources.cliff;
    let boulder_density = &resources.boulder_density;
    let boulder_scatter = &resources.boulder_scatter;
    let boulder_shape = &resources.boulder_shape;
    let tree_density = &resources.tree_density;
    let tree_scatter = &resources.tree_scatter;

    for z in 0..=size + 1 {
        for x in 0..=size + 1 {
            let wx = coord.x * CHUNK_SIZE + ((x as i32 - 1) * lod as i32);
            let wz = coord.z * CHUNK_SIZE + ((z as i32 - 1) * lod as i32);

            let mut height = 40;
            if let Some((first_noise, first_amp)) = resources.layers.first() {
                let val = {
                    let mut n = first_noise.lock().unwrap();
                    (n.get_noise_2d(wx as f32, wz as f32) + 1.0) / 2.0
                };
                height += (val * *first_amp) as i32;

                for (noise, amp) in resources.layers.iter().skip(1) {
                    let val = {
                        let mut n = noise.lock().unwrap();
                        n.get_noise_2d(wx as f32, wz as f32)
                    };
                    height += (val * *amp) as i32;
                }
            }
            let ridge = {
                let mut c = cliff.lock().unwrap();
                c.get_noise_2d(wx as f32, wz as f32).abs()
            };
            height += (ridge * 20.0) as i32;
            let height = height.clamp(1, MAX_HEIGHT - 1) as i32;
            let max_y = height + 8;

            if x >= 1 && x <= size && z >= 1 && z <= size {
                let lx = x as i32 - 1;
                let lz = z as i32 - 1;
                let occ = (lx + lz * occ_stride) as usize;
                if height > occupancy[occ] {
                    occupancy[occ] = height;
                }
            }

            for y in 1..=size + 1 {
                let wy = coord.y * CHUNK_SIZE + ((y as i32 - 1) * lod as i32);
                if wy > max_y {
                    continue;
                }

                let idx = shape.linearize([x, y, z]) as usize;
                let mut block = EMPTY;

                for offset in (0..lod).rev() {
                    let sample_y = wy + offset as i32;
                    if sample_y > max_y {
                        continue;
                    }

                    let noise = {
                        let mut c = cave.lock().unwrap();
                        c.get_noise_3d(wx as f32, sample_y as f32, wz as f32)
                    };
                    if sample_y <= height {
                        if noise > 0.8 {
                            continue;
                        }
                        block = if sample_y == height {
                            GRASS
                        } else if sample_y == height - 1 {
                            DIRT
                        } else {
                            STONE
                        };
                    } else if noise < -0.8 {
                        // block = STONE; keep this off for now, its buggy!
                    } else {
                        continue;
                    }
                    break;
                }

                if block != EMPTY {
                    voxels[idx] = block;
                }
            }

            if lod == 1 {
                let t_density = {
                    let mut n = tree_density.lock().unwrap();
                    (n.get_noise_2d(wx as f32, wz as f32) + 1.0) / 2.0
                };
                let t_scatter = {
                    let mut n = tree_scatter.lock().unwrap();
                    (n.get_noise_2d(wx as f32, wz as f32) + 1.0) / 2.0
                };
                let b_density = {
                    let mut n = boulder_density.lock().unwrap();
                    (n.get_noise_2d(wx as f32, wz as f32) + 1.0) / 2.0
                };
                let b_scatter = {
                    let mut n = boulder_scatter.lock().unwrap();
                    (n.get_noise_2d(wx as f32, wz as f32) + 1.0) / 2.0
                };
                if b_scatter < b_density * b_density * 0.3 {
                    let variant = {
                        let mut n = boulder_scatter.lock().unwrap();
                        (n.get_noise_2d(wx as f32 + 2000.0, wz as f32 + 2000.0) + 1.0) / 2.0
                    };
                    let radius = 1 + (variant * 3.0) as i32;
                    for by in 0..=radius {
                        for bx in -radius..=radius {
                            for bz in -radius..=radius {
                                let shape = {
                                    let mut s = boulder_shape.lock().unwrap();
                                    (s.get_noise_3d(
                                        (wx + bx) as f32 * 0.3,
                                        (height + by) as f32 * 0.3,
                                        (wz + bz) as f32 * 0.3,
                                    ) + 1.0)
                                        / 2.0
                                };
                                let r = (radius as f32) * (0.7 + shape * 0.6);
                                if (bx * bx + by * by + bz * bz) as f32 <= r * r {
                                    set_block(
                                        &mut voxels,
                                        wx + bx,
                                        height + by,
                                        wz + bz,
                                        STONE,
                                        &mut occupancy,
                                    );
                                }
                            }
                        }
                    }
                } else if t_scatter < t_density * t_density * 0.5 {
                    let variant = {
                        let mut n = tree_scatter.lock().unwrap();
                        (n.get_noise_2d(wx as f32 + 1000.0, wz as f32 + 1000.0) + 1.0) / 2.0
                    };
                    let trunk_size = (variant * 3.0).floor() as i32 + 1;
                    let trunk_h = 6 + trunk_size * 4 + (variant * 2.0) as i32;
                    let canopy = trunk_size * 2 + 2 + (variant * 2.0) as i32;

                    let mut colliding = false;
                    'check: for tx in 0..trunk_size {
                        for tz in 0..trunk_size {
                            let lx = ((wx + tx) - coord.x * CHUNK_SIZE) / lod as i32;
                            let lz = ((wz + tz) - coord.z * CHUNK_SIZE) / lod as i32;
                            if lx < 0 || lx >= occ_stride || lz < 0 || lz >= occ_stride {
                                continue;
                            }
                            let occ = (lx + lz * occ_stride) as usize;
                            if occupancy[occ] > height {
                                colliding = true;
                                break 'check;
                            }
                        }
                    }
                    if colliding {
                        continue;
                    }

                    for ty in 1..=trunk_h {
                        for tx in 0..trunk_size {
                            for tz in 0..trunk_size {
                                set_block(
                                    &mut voxels,
                                    wx + tx,
                                    height + ty,
                                    wz + tz,
                                    WOOD,
                                    &mut occupancy,
                                );
                            }
                        }
                    }
                    let top = height + trunk_h;
                    for dx in -canopy..=canopy {
                        for dz in -canopy..=canopy {
                            for dy in 0..=canopy {
                                if dx * dx + dz * dz + dy * dy <= canopy * canopy {
                                    set_block(
                                        &mut voxels,
                                        wx + dx,
                                        top + dy,
                                        wz + dz,
                                        LEAF,
                                        &mut occupancy,
                                    );
                                }
                            }
                        }
                    }
                }
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
                WOOD => [0.55, 0.27, 0.07, 1.0],
                LEAF => [0.2, 0.6, 0.2, 1.0],
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
