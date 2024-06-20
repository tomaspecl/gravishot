# GraviShot
This is a first person shooter game in space with asteroids and gravity. This project is still work in progress.

## Compiling
You need the Rust compiler.

You can run it with this command:

    cargo run

If you want to distibute the binary you will have to place a copy of the assets folder next to it otherwise it will crash. There is a way around it, you can embed the game assets into the binary with this command, then you can just run the binary without any other files:

    cargo run --features include_assets

You can also compile in release mode with these commands, but it will take more time and it will have to recompile:
    
    cargo run --release
    cargo run --release --features include_assets

## Controls
| Key press / Action|                                                            |
|-------------------|------------------------------------------------------------|
| WSAD              | Move forward, backward, left, right (when touching ground) |
| Q/E               | Rotate around Z-axis (when in free space)                  |
| SPACE             | Jump                                                       |
| Click mouse wheel | Switch First person/Third person                           |
| Left mouse button | Shoot (when in First person)                               |
| Mouse movement    | Look around (when in first person)                         |

For now the game starts in third person mode. In third person mode you can rotate the player by clicking the mouse and dragging. Switch to first person mode by clicking the mouse wheel. Then you can rotate just by moving the mouse. You can switch back by clicking the wheel again. When in first person mode you can shoot by clicking the left mouse button. You can move by pressing W/S/A/D when you are touching the ground (you can not move when in free space, with the exception of using the third law of motion by shooting), jump by pressing space. You can rotate around the Z axis (points out of the screen) by pressing Q/E when in free space (not touching ground).

### Temporary debug controls
You can also shoot when in third person mode by pressing G. When you are runnung the server you can enable jetpack mode by pressing J, this will allow you to move even when not touching the ground. In this mode you can also use the SHIFT key to move down (this key works even without the jetpack mode but only when on ground and thus its not very useful as it only pushes you closer to the ground a little bit).