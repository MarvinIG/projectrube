# AGENT_INFO

- Implemented a title screen with adjustable view width and exit/start buttons using the Bevy 0.16 UI system.
- Added Perlin noise terrain generation via `fastnoise-lite` when starting the game.
- Introduced game states for menu and playing, with camera controls active only during gameplay.
- Added a dedicated MenuCamera and cleanup logic for spawning and despawning the menu UI camera.
- Corrected Perlin noise frequency so terrain heights vary properly.
- Refactored gameplay, menu, player controls, and world resources into separate modules to slim down `main.rs`.
- Stacked multiple 2D Perlin noise layers for wide-spread terrain variation.
- Reintroduced 3D Perlin noise to carve sparse caves and cliffs while adding hills and plateaus.
- Switched to chunk-based world generation using 32×32×32 chunks with a configurable view width radius (default 4) and a maximum height of 128 blocks.
- Added multithreaded infinite chunk streaming with greedy meshing, frustum culling, and distance-based LOD.
- Fixed chunk gap bug by generating neighbor border voxels and upgrading nearby chunks to full resolution.
- Colored voxels: top blocks render green, subsoil brown, and underground stone gray.
