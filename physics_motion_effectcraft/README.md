# Physics Motion — EffectCraft plug-in

A deterministic spring-mesh physics warp for **EffectCraft 0.22.0**. The effect simulates a flexible image surface using fixed-step spring integration, gravity, wind, damping and edge collisions, then remaps the layer with bilinear sampling.

## What it does

- Gravity pulls the simulated surface down.
- Elasticity pulls the mesh back toward its rest shape.
- Mass changes how strongly forces accelerate the surface.
- Damping reduces oscillation over time.
- Wind and turbulence add motion.
- Bounce controls how the mesh responds at the image boundaries.
- Deformation controls the visible strength.
- Simulation Speed changes how quickly the physical motion evolves.
- Pin Top Edge anchors the upper row, creating a hanging/cloth-like motion.

The output is deterministic: the same frame time and parameters produce the same image, regardless of preview frame rate. It uses the EffectCraft WebAssembly plug-in API v1 and has no external imports.

## Build on Windows

Install Rust 1.95+ and the WebAssembly target (the included `rust-toolchain.toml` selects Rust 1.95.0):

```powershell
rustup target add wasm32-unknown-unknown
cargo build --release --locked --target wasm32-unknown-unknown
```

The plug-in will be created at:

`target\wasm32-unknown-unknown\release\physics_motion.wasm`

You can also run `build.ps1`.

## Install in EffectCraft 0.22.0

1. Open EffectCraft.
2. Choose **Effect → Load Effect Plug-in…**.
3. Select `physics_motion.wasm`.
4. Select a layer and apply **Physics Motion** from Effects & Presets → Distort.
5. Adjust its controls in Effect Controls and animate them with keyframes if desired.

To load it automatically on startup, use **Effect → Install Effect Plug-in…** and select the `.wasm`, or place it in the `Plug-ins` folder next to EffectCraft's settings folder.

## Interface note

The effect's controls are exposed as native EffectCraft parameters, so they integrate with the host's Effect Controls panel, keyframes and project saving. The WebAssembly plug-in API does not expose custom-drawn effect panels; the control layout therefore uses EffectCraft's native UI rather than pretending to provide a separate custom panel.

## Compatibility

- Target application: EffectCraft 0.22.0
- Plug-in API: WebAssembly ABI v1
- Rust: 1.95.0
- No external Rust dependencies

## Project contents

- `src/lib.rs` — effect implementation and manifest
- `Cargo.toml` — standalone crate
- `rust-toolchain.toml` — pinned toolchain and target
- `.github/workflows/build.yml` — builds the WASM artifact on GitHub Actions
