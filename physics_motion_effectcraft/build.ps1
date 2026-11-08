$ErrorActionPreference = "Stop"
rustup target add wasm32-unknown-unknown
cargo build --release --locked --target wasm32-unknown-unknown
$artifact = "target\wasm32-unknown-unknown\release\physics_motion.wasm"
if (!(Test-Path $artifact)) { throw "Build failed: $artifact was not created." }
Write-Host "Build complete: $((Resolve-Path $artifact).Path)" -ForegroundColor Green
