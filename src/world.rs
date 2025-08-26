use std::collections::HashMap;

use bevy::prelude::*;
use bevy::pbr::MeshMaterial3d;
use bevy::render::mesh::{Indices, Mesh, Mesh3d};
use bevy::render::primitives::{Aabb, Frustum};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::math::Affine3A;
use block_mesh::ndshape::{ConstShape3u32, Shape};
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG};
use fastnoise_lite::{FastNoiseLite, NoiseType};
use futures_lite::future;

use crate::player::PlayerCam;
use crate::state::AppState;

/// Size of one cubic chunk edge in blocks.
pub const CHUNK_SIZE: i32 = 32;
/// Maximum vertical height of the world in blocks.
pub const MAX_HEIGHT: i32 = 128;

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
        Self { view_width: 4 }
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
            );
    }
}

fn setup_chunk_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(Color::srgb_u8(150, 150, 150));
    commands.insert_resource(ChunkMaterial(material));
}

fn spawn_required_chunks(
    mut commands: Commands,
    params: Res<WorldParams>,
    mut pending: ResMut<PendingTasks>,
    mut map: ResMut<ChunkMap>,
    player: Query<&Transform, With<PlayerCam>>,
    chunks: Query<&Chunk>,
) {
    let pool = AsyncComputeTaskPool::get();
    let player_pos = player
        .single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    let player_chunk = IVec3::new(
        (player_pos.x / CHUNK_SIZE as f32).floor() as i32,
        0,
        (player_pos.z / CHUNK_SIZE as f32).floor() as i32,
    );

    // Despawn chunks far outside the view radius
    let mut to_remove = Vec::new();
    for (coord, entity) in map.entities.iter() {
        let dist = (coord.x - player_chunk.x).abs().max((coord.z - player_chunk.z).abs());
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
            let coord = player_chunk + IVec3::new(x, 0, z);
            let dist = x.abs().max(z.abs());
            let required_lod = if dist <= 3 { 1 } else { 2 };

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

            let task = pool.spawn(async move {
                let mesh = generate_chunk_mesh(coord, required_lod);
                (coord, required_lod, mesh)
            });
            pending.tasks.insert(coord, (required_lod, task));
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
struct BoolVoxel(bool);

const EMPTY: BoolVoxel = BoolVoxel(false);
const FULL: BoolVoxel = BoolVoxel(true);

impl Voxel for BoolVoxel {
    fn get_visibility(&self) -> VoxelVisibility {
        if *self == EMPTY {
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        }
    }
}

impl MergeVoxel for BoolVoxel {
    type MergeValue = BoolVoxel;
    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}

fn generate_chunk_mesh(coord: IVec3, lod: u32) -> Mesh {
    match lod {
        1 => build_mesh::<{ CHUNK_SIZE_U32 + 3 }>(coord, lod),
        2 => build_mesh::<{ LOD2_SIZE_U32 + 3 }>(coord, lod),
        _ => build_mesh::<{ CHUNK_SIZE_U32 + 3 }>(coord, 1),
    }
}

fn build_mesh<const N: u32>(coord: IVec3, lod: u32) -> Mesh {
    let size = N - 2;

    let shape = ConstShape3u32::<{ N }, { N }, { N }> {};
    let mut voxels = vec![EMPTY; (N * N * N) as usize];

    let mut base = FastNoiseLite::with_seed(0);
    base.set_noise_type(Some(NoiseType::Perlin));
    base.set_frequency(Some(0.005));

    let mut detail = FastNoiseLite::with_seed(1);
    detail.set_noise_type(Some(NoiseType::Perlin));
    detail.set_frequency(Some(0.02));

    for z in 0..=size + 1 {
        for x in 0..=size + 1 {
            let wx = coord.x * CHUNK_SIZE + ((x as i32 - 1) * lod as i32);
            let wz = coord.z * CHUNK_SIZE + ((z as i32 - 1) * lod as i32);
            let base_val = base.get_noise_2d(wx as f32, wz as f32);
            let detail_val = detail.get_noise_2d(wx as f32, wz as f32);
            let height = ((base_val * 20.0) + (detail_val * 5.0) + 20.0)
                .round()
                .clamp(1.0, (MAX_HEIGHT - 1) as f32) as i32;
            for y in 0..=size + 1 {
                let wy = coord.y * CHUNK_SIZE + ((y as i32 - 1) * lod as i32);
                if wy <= height {
                    let idx = shape.linearize([x, y, z]) as usize;
                    voxels[idx] = FULL;
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
    let mut indices: Vec<u32> = Vec::new();

    for (face, group) in RIGHT_HANDED_Y_UP_CONFIG
        .faces
        .iter()
        .zip(buffer.quads.groups.iter())
    {
        for quad in group.iter() {
            let start = positions.len() as u32;
            positions.extend_from_slice(&face.quad_mesh_positions(quad, lod as f32));
            normals.extend_from_slice(&face.quad_mesh_normals());
            indices.extend_from_slice(&face.quad_mesh_indices(start));
        }
    }

    use bevy::render::mesh::PrimitiveTopology;
    use bevy::render::render_asset::RenderAssetUsages;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

