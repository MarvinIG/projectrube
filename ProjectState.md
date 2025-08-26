# Project State

## Finished
- Basic voxel world rendering with free camera controls.
- Title screen with adjustable view width, Start Game and Exit buttons.
- Perlin noise terrain generation on game start with corrected frequency for varied height.
- World generation uses 32×32×32 chunks with a configurable view width radius and a maximum height of 128 blocks, combining stacked 2D noise with 3D noise caves for hills and plateaus.
- Chunks stream infinitely as the player moves, meshed with a greedy algorithm and culled via camera frustum with distance-based LOD.
- Nearby chunks automatically regenerate at full resolution and border voxels are populated to eliminate seams between chunks, fixing the previous chunk gap bug.
- Surface blocks render green, the layer below brown, and deeper blocks gray.
- Terrain noise retuned for noticeable hills and plateaus, and chunk mesh positions corrected so all faces render.
- Terrain height now stacks five configurable noise layers adjustable from the title screen and saved to `settings.json`.
- Pressing `P` in-game returns to the title screen and removes active world entities.

## WIP
- None
