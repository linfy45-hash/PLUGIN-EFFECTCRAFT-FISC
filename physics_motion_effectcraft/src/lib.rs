//! Physics Motion — deterministic spring-mesh image deformation for EffectCraft.
//! WebAssembly plug-in API v1. No imports, clock, randomness, or external dependencies.

use std::cell::RefCell;

const GRID: usize = 16;
const SIDE: usize = GRID + 1;
const NODES: usize = SIDE * SIDE;
const MANIFEST: &str = r#"{
  "api": 1,
  "id": "io.github.effectcraft-physics-motion",
  "name": "Physics Motion",
  "category": "Distort",
  "version": "1.0.0",
  "author": "Independent developer",
  "description": "Deforms a layer with a deterministic spring-mesh simulation driven by gravity, wind and elastic forces.",
  "params": [
    {"id":"gravity","name":"Gravity","type":"slider","default":0.45,"min":0.0,"max":3.0,"decimals":2},
    {"id":"elasticity","name":"Elasticity","type":"slider","default":5.0,"min":0.2,"max":18.0,"decimals":1},
    {"id":"mass","name":"Mass","type":"slider","default":1.0,"min":0.2,"max":5.0,"decimals":2},
    {"id":"damping","name":"Damping","type":"slider","default":2.8,"min":0.0,"max":12.0,"decimals":1},
    {"id":"wind","name":"Wind Force","type":"slider","default":0.35,"min":-2.0,"max":2.0,"decimals":2},
    {"id":"wind_speed","name":"Wind Speed","type":"slider","default":1.4,"min":0.0,"max":8.0,"decimals":1},
    {"id":"turbulence","name":"Turbulence","type":"slider","default":0.12,"min":0.0,"max":1.0,"decimals":2},
    {"id":"bounce","name":"Bounce","type":"slider","default":0.35,"min":0.0,"max":1.0,"decimals":2},
    {"id":"strength","name":"Deformation","type":"slider","default":100.0,"min":0.0,"max":200.0,"decimals":0},
    {"id":"speed","name":"Simulation Speed","type":"slider","default":1.0,"min":0.0,"max":3.0,"decimals":2},
    {"id":"pin_top","name":"Pin Top Edge","type":"checkbox","default":false}
  ]
}"#;

#[derive(Clone, Copy, Default)]
struct Node {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

thread_local! {
    static TRANSFER: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static SCRATCH: RefCell<Vec<[f32; 4]>> = const { RefCell::new(Vec::new()) };
    static MESH: RefCell<Vec<Node>> = RefCell::new(vec![Node::default(); NODES]);
}

#[unsafe(no_mangle)]
pub extern "C" fn ec_api_version() -> i32 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn ec_manifest_ptr() -> i32 { MANIFEST.as_ptr() as i32 }

#[unsafe(no_mangle)]
pub extern "C" fn ec_manifest_len() -> i32 { MANIFEST.len() as i32 }

#[unsafe(no_mangle)]
pub extern "C" fn ec_alloc(bytes: i32) -> i32 {
    TRANSFER.with(|b| {
        let mut b = b.borrow_mut();
        let words = (bytes.max(0) as usize).div_ceil(8);
        if b.len() < words { b.resize(words, 0); }
        b.as_mut_ptr() as i32
    })
}

fn clamp01(v: f32) -> f32 { v.clamp(0.0, 1.0) }

fn node_index(x: usize, y: usize) -> usize { y * SIDE + x }

fn simulate(mesh: &mut [Node], time: f32, p: &[f64]) {
    let gravity = p[0].clamp(0.0, 3.0) as f32;
    let elasticity = p[1].clamp(0.2, 18.0) as f32;
    let mass = (p[2].clamp(0.2, 5.0) as f32).max(0.01);
    let damping = p[3].clamp(0.0, 12.0) as f32;
    let wind = p[4].clamp(-2.0, 2.0) as f32;
    let wind_speed = p[5].clamp(0.0, 8.0) as f32;
    let turbulence = p[6].clamp(0.0, 1.0) as f32;
    let bounce = p[7].clamp(0.0, 1.0) as f32;
    let pin_top = p[10] >= 0.5;

    for y in 0..SIDE {
        for x in 0..SIDE {
            let i = node_index(x, y);
            mesh[i] = Node {
                x: x as f32 / GRID as f32,
                y: y as f32 / GRID as f32,
                vx: 0.0,
                vy: 0.0,
            };
        }
    }

    // Fixed-step integration makes the result independent of preview frame rate.
    let total_steps = ((time.max(0.0) * 60.0).ceil() as usize).min(1800);
    let dt = 1.0 / 60.0;
    for step in 0..total_steps {
        let t = (step as f32 + 1.0) * dt;
        let wind_phase = t * wind_speed * 2.0;
        let mut forces = [(0.0f32, 0.0f32); NODES];

        for gy in 0..SIDE {
            for gx in 0..SIDE {
                let i = node_index(gx, gy);
                let n = mesh[i];
                let rx = gx as f32 / GRID as f32;
                let ry = gy as f32 / GRID as f32;

                // Soft attachment to each node's rest position.
                let mut fx = (rx - n.x) * elasticity * 0.30;
                let mut fy = (ry - n.y) * elasticity * 0.30 + gravity * 0.16;

                // Neighbour springs preserve the shape while allowing it to flex.
                for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
                    let nx = gx as isize + dx;
                    let ny = gy as isize + dy;
                    if nx < 0 || ny < 0 || nx >= SIDE as isize || ny >= SIDE as isize { continue; }
                    let j = node_index(nx as usize, ny as usize);
                    let other = mesh[j];
                    let rest_x = dx as f32 / GRID as f32;
                    let rest_y = dy as f32 / GRID as f32;
                    fx += ((other.x - n.x) - rest_x) * elasticity * 0.20;
                    fy += ((other.y - n.y) - rest_y) * elasticity * 0.20;
                }

                let wave = (wind_phase + ry * 5.0 + rx * 2.1).sin();
                let ripple = (t * 4.7 + rx * 11.0 - ry * 8.0).sin();
                fx += wind * wave * (0.35 + ry * 0.65);
                fy += turbulence * ripple * 0.16;
                forces[i] = (fx / mass, fy / mass);
            }
        }

        let drag = (-damping * dt).exp();
        for gy in 0..SIDE {
            for gx in 0..SIDE {
                let i = node_index(gx, gy);
                if pin_top && gy == 0 {
                    mesh[i].x = gx as f32 / GRID as f32;
                    mesh[i].y = 0.0;
                    mesh[i].vx = 0.0;
                    mesh[i].vy = 0.0;
                    continue;
                }
                let (ax, ay) = forces[i];
                let n = &mut mesh[i];
                n.vx = (n.vx + ax * dt) * drag;
                n.vy = (n.vy + ay * dt) * drag;
                n.x += n.vx * dt;
                n.y += n.vy * dt;

                if n.x < 0.0 { n.x = 0.0; n.vx = n.vx.abs() * bounce; }
                if n.x > 1.0 { n.x = 1.0; n.vx = -n.vx.abs() * bounce; }
                if n.y < 0.0 { n.y = 0.0; n.vy = n.vy.abs() * bounce; }
                if n.y > 1.0 { n.y = 1.0; n.vy = -n.vy.abs() * bounce; }
            }
        }
    }
}

fn displacement(mesh: &[Node], u: f32, v: f32) -> (f32, f32) {
    let gx = clamp01(u) * GRID as f32;
    let gy = clamp01(v) * GRID as f32;
    let x0 = (gx.floor() as usize).min(GRID - 1);
    let y0 = (gy.floor() as usize).min(GRID - 1);
    let tx = gx - x0 as f32;
    let ty = gy - y0 as f32;

    let a = mesh[node_index(x0, y0)];
    let b = mesh[node_index(x0 + 1, y0)];
    let c = mesh[node_index(x0, y0 + 1)];
    let d = mesh[node_index(x0 + 1, y0 + 1)];

    let dx0 = (b.x - (x0 + 1) as f32 / GRID as f32) * (1.0 - tx)
        + (a.x - x0 as f32 / GRID as f32) * tx;
    let dx1 = (d.x - (x0 + 1) as f32 / GRID as f32) * (1.0 - tx)
        + (c.x - x0 as f32 / GRID as f32) * tx;
    let dy0 = (b.y - y0 as f32 / GRID as f32) * (1.0 - tx)
        + (a.y - y0 as f32 / GRID as f32) * tx;
    let dy1 = (d.y - y0 as f32 / GRID as f32) * (1.0 - tx)
        + (c.y - y0 as f32 / GRID as f32) * tx;

    (dx0 * (1.0 - ty) + dx1 * ty, dy0 * (1.0 - ty) + dy1 * ty)
}

fn sample(src: &[[f32; 4]], w: usize, h: usize, x: f32, y: f32) -> [f32; 4] {
    if x < 0.0 || y < 0.0 || x > (w - 1) as f32 || y > (h - 1) as f32 {
        return [0.0; 4];
    }
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let a = src[y0 * w + x0];
    let b = src[y0 * w + x1];
    let c = src[y1 * w + x0];
    let d = src[y1 * w + x1];
    let mut out = [0.0; 4];
    for k in 0..4 {
        let top = a[k] * (1.0 - tx) + b[k] * tx;
        let bottom = c[k] * (1.0 - tx) + d[k] * tx;
        out[k] = top * (1.0 - ty) + bottom * ty;
    }
    out
}

/// # Safety
/// The host passes valid pointers into the transfer buffer returned by `ec_alloc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ec_render(
    pixels: i32, width: i32, height: i32, params: i32, nparams: i32,
    time: f64, _scale: f64,
) -> i32 {
    if nparams < 11 || width <= 0 || height <= 0 || !time.is_finite() { return 1; }
    let w = width as usize;
    let h = height as usize;
    let Some(count) = w.checked_mul(h) else { return 1; };
    if count == 0 || count > 100_000_000 { return 1; }

    let p = unsafe { std::slice::from_raw_parts(params as *const f64, nparams as usize) };
    let px = unsafe { std::slice::from_raw_parts_mut(pixels as *mut [f32; 4], count) };
    let sim_time = (time.max(0.0) * p[9].clamp(0.0, 3.0)).min(30.0) as f32;
    if sim_time <= 0.0001 || p[8] <= 0.0 { return 0; }

    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        if scratch.len() < count { scratch.resize(count, [0.0; 4]); }
        scratch[..count].copy_from_slice(px);

        MESH.with(|mesh_cell| {
            let mut mesh = mesh_cell.borrow_mut();
            simulate(&mut mesh, sim_time, p);
            let strength = (p[8].clamp(0.0, 200.0) as f32) / 100.0;

            for y in 0..h {
                for x in 0..w {
                    let u = if w > 1 { x as f32 / (w - 1) as f32 } else { 0.5 };
                    let v = if h > 1 { y as f32 / (h - 1) as f32 } else { 0.5 };
                    let (dx, dy) = displacement(&mesh, u, v);
                    let sx = x as f32 - dx * w as f32 * strength;
                    let sy = y as f32 - dy * h as f32 * strength;
                    px[y * w + x] = sample(&scratch[..count], w, h, sx, sy);
                }
            }
        });
    });
    0
}
