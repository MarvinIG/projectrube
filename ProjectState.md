# Project State

## Finished
- Basic voxel world rendering with free camera controls.
- Title screen with adjustable view width, Start Game and Exit buttons.
- Perlin noise terrain generation on game start with corrected frequency for varied height.
- World generation uses 32×32×32 chunks with a configurable view width radius and a maximum height of 256 blocks, combining stacked 2D noise with 3D noise caves for hills and plateaus.
- Chunks stream infinitely as the player moves, meshed with a greedy algorithm and culled via camera frustum with distance-based LOD.
- Nearby chunks automatically regenerate at full resolution and border voxels are populated to eliminate seams between chunks, fixing the previous chunk gap bug.
- Surface blocks render green, the layer below brown, and deeper blocks gray.
- Terrain noise retuned for noticeable hills and plateaus, and chunk mesh positions corrected so all faces render.
- Terrain height now stacks five configurable noise layers adjustable from the title screen and saved to `settings.json`.
- Pressing `P` in-game returns to the title screen and removes active world entities.
- Chunks retain full detail within an eight-chunk radius, and distant low-detail meshes sample the surface block so colors remain accurate when approached.
- Chunks now spawn in stacked vertical layers up to eight chunks high, enabling a fully 3D world grid.
- Ridged noise adds cliffs and overhangs while boulder generation now uses density and scatter noise with irregular shapes, and trees vary trunk size with collision-safe placement.
- Last processed player chunk cached so spawn/despawn logic runs only when moving between chunks.
- Noise generators created once and shared through a `NoiseResources` Bevy resource, eliminating repeated `FastNoiseLite` setup.
- Tree spawning consults a per-column occupancy map instead of scanning every trunk voxel, reducing generation cost.

## WIP
- None
