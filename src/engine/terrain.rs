//! The one custom shader in the project.
//!
//! `ColorMaterial` stays the base material, so Bevy keeps owning the mesh2d
//! vertex pipeline, the bind group layout, tonemapping and alpha handling. The
//! extension replaces only the fragment stage with `assets/shaders/terrain.wesl`,
//! which adds a slow drift of light over the baked terrain texture.
//!
//! Keeping the custom shader in this one place means a shader that fails to
//! compile can be reverted to a plain `ColorMaterial` without touching the rest
//! of the renderer.

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::{Shader, ShaderRef};
use bevy::sprite_render::{
    ColorMaterial, ExtendedMaterial2d, Material2dPlugin, MaterialExtension2d,
};
use std::sync::OnceLock;

/// The shader source, compiled into the binary. Keeping it embedded means the
/// game renders identically no matter where the executable is started from -
/// no asset directory has to be found at runtime.
const TERRAIN_WESL: &str = include_str!("../../assets/shaders/terrain.wesl");

/// Set once at startup, before any material is specialized.
static TERRAIN_SHADER: OnceLock<Handle<Shader>> = OnceLock::new();

/// Shader parameters, kept as a single `vec4` so the Rust and WGSL layouts
/// cannot drift apart.
///
/// * `x` - seconds of world time (frozen while paused, faster at 4x speed)
/// * `y` - map width in tiles
/// * `z` - map height in tiles
/// * `w` - shimmer strength
#[derive(Clone, Debug, ShaderType)]
pub struct Weather {
    pub data: Vec4,
}

impl Weather {
    /// Parameters for a map of `cols` x `rows` tiles.
    pub fn for_map(cols: f32, rows: f32) -> Self {
        Self {
            data: Vec4::new(0.0, cols, rows, 0.035),
        }
    }
}

/// The extension: one uniform buffer, read by the shader at binding 3.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WeatherExtension {
    #[uniform(3)]
    pub params: Weather,
}

impl MaterialExtension2d for WeatherExtension {
    fn fragment_shader() -> Option<ShaderRef> {
        Some(ShaderRef::Handle(
            TERRAIN_SHADER.get().cloned().unwrap_or_default(),
        ))
    }
}

/// `ColorMaterial` + the weather pass.
pub type TerrainMaterial = ExtendedMaterial2d<ColorMaterial, WeatherExtension>;

/// Registers the extended 2D material and its shader.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<TerrainMaterial>::default())
            .add_systems(Startup, load_shader);
    }
}

/// Publish the embedded source as a shader asset. `Startup` runs well before
/// the first specialization pass, so materials always see the real handle.
fn load_shader(mut shaders: ResMut<Assets<Shader>>) {
    let handle = shaders.add(Shader::from_wesl(
        TERRAIN_WESL,
        "assets/shaders/terrain.wesl",
    ));
    let _ = TERRAIN_SHADER.set(handle);
}
