//! Procedural map generation: layered value-noise terrain, mineral deposits,
//! and A* road pathfinding around impassable ground.

use super::types::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Value noise (seeded, deterministic, zero dependencies)
// ---------------------------------------------------------------------------

/// Smoothstep interpolation.
fn fade(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// A tiny deterministic hash -> [0, 1) for lattice corners.
fn lattice_hash(seed: u64, x: i32, y: i32) -> f32 {
    let mut h = seed
        ^ (x as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (y as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D049BB133111EB);
    h = h ^ (h >> 31);
    (h & 0xFFFFFF) as f32 / 0x1000000 as f32
}

/// Single-octave value noise sampled at float coordinates.
fn value_noise(seed: u64, x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;

    let v00 = lattice_hash(seed, xi, yi);
    let v10 = lattice_hash(seed, xi + 1, yi);
    let v01 = lattice_hash(seed, xi, yi + 1);
    let v11 = lattice_hash(seed, xi + 1, yi + 1);

    let sx = fade(xf);
    let sy = fade(yf);

    let a = v00 + (v10 - v00) * sx;
    let b = v01 + (v11 - v01) * sx;
    a + (b - a) * sy
}

/// Fractal Brownian motion: layered noise for natural-looking terrain.
fn fbm(seed: u64, x: f32, y: f32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut max_amp = 0.0;

    for i in 0..octaves {
        value += value_noise(seed.wrapping_add(i as u64 * 7919), x * frequency, y * frequency) * amplitude;
        max_amp += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_amp
}

// ---------------------------------------------------------------------------
// Map
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub cols: i32,
    pub rows: i32,
    pub seed: u64,
    pub terrain: Vec<Terrain>,
    pub deposit: Vec<Deposit>,
    /// Static per-tile brightness jitter for visual richness, 0.85..1.15.
    pub shade: Vec<f32>,
}

impl Map {
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.cols + x) as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.cols && y < self.rows
    }

    pub fn terrain_at(&self, x: i32, y: i32) -> Terrain {
        if self.in_bounds(x, y) {
            self.terrain[self.idx(x, y)]
        } else {
            Terrain::Water
        }
    }

    pub fn deposit_at(&self, x: i32, y: i32) -> Deposit {
        if self.in_bounds(x, y) {
            self.deposit[self.idx(x, y)]
        } else {
            Deposit::None
        }
    }

    pub fn passable(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.terrain_at(x, y).passable()
    }

    /// Distance between two nodes in world (tile) units.
    pub fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
        ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
    }

    /// Generate a fresh map from a seed.
    ///
    /// Layers:
    ///   1. elevation = fbm → water / sand / grass / mountain
    ///   2. moisture  = fbm, shifted → forest on wet grass
    ///   3. deposits  → iron near mountains, copper in pockets
    ///   4. shade     → per-tile brightness variation
    pub fn generate(seed: u64, cols: i32, rows: i32) -> Map {
        let mut terrain = vec![Terrain::Grass; (cols * rows) as usize];
        let mut deposit = vec![Deposit::None; (cols * rows) as usize];
        let mut shade = vec![1.0f32; (cols * rows) as usize];

        let scale = 0.09; // features roughly every 11 tiles

        for y in 0..rows {
            for x in 0..cols {
                let i = (y * cols + x) as usize;
                let fx = x as f32 * scale;
                let fy = y as f32 * scale;

                let elev = fbm(seed, fx, fy, 4);
                let moist = fbm(seed ^ 0xA5A5_A5A5, fx * 1.3 + 40.0, fy * 1.3 + 40.0, 3);
                let detail = fbm(seed ^ 0x5A5A_5A5A, fx * 4.0, fy * 4.0, 2);

                terrain[i] = if elev < 0.34 {
                    Terrain::Water
                } else if elev < 0.38 {
                    Terrain::Sand
                } else if elev > 0.78 {
                    Terrain::Mountain
                } else if moist > 0.63 {
                    Terrain::Forest
                } else {
                    Terrain::Grass
                };

                // Iron in highlands, copper in mid-elevation pockets.
                if terrain[i] == Terrain::Mountain && detail > 0.62 {
                    deposit[i] = Deposit::Iron;
                } else if matches!(terrain[i], Terrain::Grass | Terrain::Sand) && detail < 0.30 {
                    deposit[i] = Deposit::Copper;
                }

                // Brightness jitter: subtle hash-based per-tile variation.
                shade[i] = 0.88 + 0.24 * lattice_hash(seed ^ 0xD1B5_4A32, x, y);
            }
        }

        // Soften isolated single-tile anomalies (1-tile ponds / pillars).
        terrain = smooth_isolates(terrain, cols, rows);

        Map {
            cols,
            rows,
            seed,
            terrain,
            deposit,
            shade,
        }
    }

    /// Find a buildable tile near the requested spot (for capital placement).
    ///
    /// Never returns water or an off-map tile: a capital stranded on a water
    /// tile or at the raw map edge can never be fed and starves to zero
    /// population, which used to be an unrecoverable start.
    pub fn nearest_passable(&self, x: i32, y: i32, min_dist_from_water: i32) -> (i32, i32) {
        if let Some(p) = self.find_open_tile(x, y, min_dist_from_water, 40) {
            return p;
        }
        // Relax the clearance requirement first, then the terrain one, scanning
        // in expanding rings so we always take the closest workable tile.
        if let Some(p) = self.find_open_tile(x, y, 1, self.cols.max(self.rows)) {
            return p;
        }
        if let Some(p) = self.nearest_scan(x, y, |t| t.passable()) {
            return p;
        }
        // Degenerate map: stay in bounds no matter what.
        (
            x.clamp(0, (self.cols - 1).max(0)),
            y.clamp(0, (self.rows - 1).max(0)),
        )
    }

    /// Closest in-bounds tile matching `ok`, walking rings outwards.
    fn nearest_scan(&self, cx: i32, cy: i32, ok: impl Fn(Terrain) -> bool) -> Option<(i32, i32)> {
        for r in 0..=self.cols.max(self.rows) {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if self.in_bounds(x, y) && ok(self.terrain_at(x, y)) {
                        return Some((x, y));
                    }
                }
            }
        }
        None
    }

    fn find_open_tile(&self, cx: i32, cy: i32, min_clear: i32, max_radius: i32) -> Option<(i32, i32)> {
        for r in 0..=max_radius {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let x = cx + dx;
                    let y = cy + dy;
                    if !self.in_bounds(x, y) {
                        continue;
                    }
                    if self.terrain_at(x, y) != Terrain::Grass {
                        continue;
                    }
                    if self.clearance(x, y) >= min_clear {
                        return Some((x, y));
                    }
                }
            }
        }
        None
    }

    /// How many tiles in a square ring are land (approx open-space measure).
    fn clearance(&self, x: i32, y: i32) -> i32 {
        let mut n = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if self.passable(x + dx, y + dy) {
                    n += 1;
                }
            }
        }
        n
    }

    // -----------------------------------------------------------------
    // A* pathfinding for roads
    // -----------------------------------------------------------------

    /// Route a road from A to B around water and mountains.
    /// Returns the full tile path including endpoints, or None if blocked
    /// or absurdly long.
    pub fn find_road(&self, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
        if !self.passable(from.0, from.1) || !self.passable(to.0, to.1) {
            return None;
        }
        if from == to {
            return Some(vec![from]);
        }

        // Budget in edge-cost units (heuristic scale: 10 per orthogonal
        // step), allowing the path to wind up to MAX_PATH_FACTOR× longer
        // than the direct route.
        let straight = Self::heuristic(from, to);
        let max_cost = (straight as f32 * MAX_PATH_FACTOR) as i32 + 30;

        // Binary heap of (f_score, node). i32 keys keep it Ord.
        let mut open: std::collections::BinaryHeap<(std::cmp::Reverse<i32>, (i32, i32))> =
            std::collections::BinaryHeap::new();
        let mut g_score: std::collections::HashMap<(i32, i32), i32> =
            std::collections::HashMap::new();
        let mut came_from: std::collections::HashMap<(i32, i32), (i32, i32)> =
            std::collections::HashMap::new();

        g_score.insert(from, 0);
        open.push((std::cmp::Reverse(Self::heuristic(from, to)), from));

        let dirs = [
            (1, 0, 10),
            (-1, 0, 10),
            (0, 1, 10),
            (0, -1, 10),
            (1, 1, 14),
            (1, -1, 14),
            (-1, 1, 14),
            (-1, -1, 14),
        ];

        let mut iterations = 0;
        while let Some((_, current)) = open.pop() {
            iterations += 1;
            if iterations > 20_000 {
                return None;
            }

            if current == to {
                return Some(Self::reconstruct(&came_from, current));
            }

            let current_g = *g_score.get(&current).unwrap_or(&i32::MAX);
            if current_g > max_cost {
                continue;
            }

            for (dx, dy, cost) in dirs {
                let next = (current.0 + dx, current.1 + dy);
                if !self.passable(next.0, next.1) {
                    continue;
                }
                // Diagonal moves must not cut a corner between two blocked tiles.
                if dx != 0 && dy != 0 {
                    if !self.passable(current.0 + dx, current.1)
                        || !self.passable(current.0, current.1 + dy)
                    {
                        continue;
                    }
                }

                let tentative = current_g + cost;
                if tentative < *g_score.get(&next).unwrap_or(&i32::MAX) {
                    came_from.insert(next, current);
                    g_score.insert(next, tentative);
                    let f = tentative + Self::heuristic(next, to);
                    open.push((std::cmp::Reverse(f), next));
                }
            }
        }
        None
    }

    fn heuristic(a: (i32, i32), b: (i32, i32)) -> i32 {
        let dx = (a.0 - b.0).abs();
        let dy = (a.1 - b.1).abs();
        10 * (dx + dy) - 6 * dx.min(dy) // octile distance
    }

    /// A straight line of tile steps from `a` to `b`, one point per tile.
    ///
    /// Used when A* cannot route (the map blocks the way): a route that cuts
    /// across rough ground still carries caravans and peasants, and its length
    /// stays proportional to the real distance.
    pub fn straight_path(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
        let dx = (b.0 - a.0) as f32;
        let dy = (b.1 - a.1) as f32;
        let steps = (dx * dx + dy * dy).sqrt().ceil().max(1.0) as i32;
        (0..=steps)
            .map(|i| {
                let t = i as f32 / steps as f32;
                (
                    a.0 + (dx * t).round() as i32,
                    a.1 + (dy * t).round() as i32,
                )
            })
            .collect()
    }

    fn reconstruct(
        came_from: &std::collections::HashMap<(i32, i32), (i32, i32)>,
        mut current: (i32, i32),
    ) -> Vec<(i32, i32)> {
        let mut path = vec![current];
        while let Some(&prev) = came_from.get(&current) {
            current = prev;
            path.push(current);
        }
        path.reverse();
        path
    }
}

/// Replace tiles that are completely surrounded by a different terrain
/// (single-tile ponds, lone pillars) with the dominant neighbor.
fn smooth_isolates(terrain: Vec<Terrain>, cols: i32, rows: i32) -> Vec<Terrain> {
    let mut out = terrain.clone();
    let at = |t: &Vec<Terrain>, x: i32, y: i32| -> Terrain {
        if x >= 0 && y >= 0 && x < cols && y < rows {
            t[(y * cols + x) as usize]
        } else {
            Terrain::Water
        }
    };

    for y in 0..rows {
        for x in 0..cols {
            let i = (y * cols + x) as usize;
            let t = terrain[i];
            let mut same = 0;
            let mut total = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    total += 1;
                    if at(&terrain, x + dx, y + dy) == t {
                        same += 1;
                    }
                }
            }
            if same <= 1 && total == 8 {
                // Isolated: take the most common orthogonal neighbor.
                let mut best = t;
                let mut best_count = 0;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = at(&terrain, x + dx, y + dy);
                    let c = terrain
                        .iter()
                        .filter(|&&tt| tt == n)
                        .count();
                    if c > best_count {
                        best_count = c;
                        best = n;
                    }
                }
                out[i] = best;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Place-name generation (flavour without asset files)
// ---------------------------------------------------------------------------

const NAME_PREFIX: &[&str] = &[
    "Silk", "Ash", "Velvet", "Thorn", "Ember", "Gloom", "Bright", "Iron", "Rune", "Mist",
    "Cinder", "Duskmere", "Gold", "Hollow", "Web", "Storm", "Frost", "Bramble", "Night", "Sun",
    "Pearl", "Rust", "Wyrm", "Glass", "Crow", "Thistle", "Moon", "Salt", "Oak", "Shade",
];

const NAME_SUFFIX: &[&str] = &[
    "hold", "mere", "gate", "fen", "spire", "rest", "watch", "ford", "hollow", "reach",
    "mark", "vale", "burg", "wick", "holt", "stead", "ford", "crest", "dell", "moor",
];

const SPY_FIRST: &[&str] = &[
    "Vex", "Sil", "Nyx", "Kara", "Dren", "Ilo", "Fen", "Mara", "Tess", "Orin",
    "Shae", "Volk", "Enna", "Rook", "Lys", "Cade", "Mira", "Zeph", "Bryn", "Hale",
];

const SPY_LAST: &[&str] = &[
    "the Quiet", "Needle", "Whisper", "Shade", "Coil", "Thread", "Gloom", "Veil",
    "Echo", "Strand", "Wisp", "Silkhand", "Shadow", "in the Dark", "the Patient",
];

/// Generate a settlement or faction place name from a seed.
pub fn place_name(seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let a = NAME_PREFIX[rng.index(NAME_PREFIX.len())];
    let b = NAME_SUFFIX[rng.index(NAME_SUFFIX.len())];
    format!("{}{}", a, b)
}

/// Generate an agent name for spies and workers.
pub fn agent_name(seed: u64) -> String {
    let mut rng = Rng::new(seed ^ 0xBEEF);
    let f = SPY_FIRST[rng.index(SPY_FIRST.len())];
    let l = SPY_LAST[rng.index(SPY_LAST.len())];
    format!("{} {}", f, l)
}
