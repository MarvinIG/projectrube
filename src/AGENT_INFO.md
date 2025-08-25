# AGENT_INFO

- Implemented a title screen with adjustable world size and exit/start buttons using the Bevy 0.16 UI system.
- Added Perlin noise terrain generation via `fastnoise-lite` when starting the game.
- Introduced game states for menu and playing, with camera controls active only during gameplay.
- Added a dedicated MenuCamera and cleanup logic for spawning and despawning the menu UI camera.
