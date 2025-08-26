# AGENT_INFO

- Implemented a title screen with adjustable view width and exit/start buttons using the Bevy 0.16 UI system.
- Added Perlin noise terrain generation via `fastnoise-lite` when starting the game.
- Introduced game states for menu and playing, with camera controls active only during gameplay.
- Added a dedicated MenuCamera and cleanup logic for spawning and despawning the menu UI camera.
- Corrected Perlin noise frequency so terrain heights vary properly.
- Refactored gameplay, menu, player controls, and world resources into separate modules to slim down `main.rs`.
- Stacked multiple 2D Perlin noise layers for wide-spread terrain variation.
- Applied 3D Perlin noise to carve caves, cliffs, and ravines without exposing void spaces.
- Switched to chunk-based world generation using 32×32×32 chunks with a configurable view width radius (default 4) and a maximum height of 128 blocks.
- Added multithreaded infinite chunk streaming with greedy meshing, frustum culling, and distance-based LOD.
- Chunks now upgrade to full resolution within three chunks of the player and generate neighbor border voxels to remove gaps between chunks.
