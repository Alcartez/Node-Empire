//! Node Empire terrain shader.
//!
//! This is Bevy's `color_material.wesl` fragment stage with one addition: a
//! slow "weather" pass that drifts over the baked terrain texture. The vertex
//! stage, the bind group layout, tonemapping and alpha handling all stay
//! Bevy's, which keeps the custom part small and easy to revert.

import bevy_sprite_render::mesh2d::{
    vertex_output::VertexOutput,
    view_bindings::view,
    bindings::mesh,
};
import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d};

@if(TONEMAP_IN_SHADER)
import bevy_core_pipeline::tonemapping;
@if(SRGB_OUTPUT)
import bevy_render::color_operations::linear_to_srgb;
@if(OKLAB_OUTPUT)
import bevy_render::color_operations::linear_rgb_to_oklab;

// ---------------------------------------------------------------------------
// Base ColorMaterial bindings (identical to bevy_sprite_render::mesh2d)
// ---------------------------------------------------------------------------

struct ColorMaterial {
    color: vec4<f32>,
    uv_transform: mat3x3<f32>,
    /// 'flags' is a bit field indicating various options.
    flags: u32,
    alpha_cutoff: f32,
};

@if(BINDLESS) {
/// The index table for bindless operation.
struct ColorMaterialBindings {
    material: u32,          // 0
    color_texture: u32,     // 1
    color_sampler: u32,     // 2
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<storage> color_material_indices:
    array<ColorMaterialBindings>;
@group(constants::MATERIAL_BIND_GROUP) @binding(10) var<storage> color_materials: array<ColorMaterial>;
}
@else {
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: ColorMaterial;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var texture_sampler: sampler;
}

// ---------------------------------------------------------------------------
// Extension bindings. ColorMaterial itself only uses 0..=2, so 3 is ours.
// ---------------------------------------------------------------------------

/// `x` = seconds, `y`/`z` = map size in tiles, `w` = shimmer strength.
struct Weather {
    data: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> weather: Weather;

const COLOR_MATERIAL_FLAGS_TEXTURE_BIT: u32              = 1u;
const COLOR_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS: u32 = 3221225472u; // (0b11u32 << 30)
const COLOR_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32        = 0u;          // (0u32 << 30)
const COLOR_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32          = 1073741824u; // (1u32 << 30)
const COLOR_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32         = 2147483648u; // (2u32 << 30)

@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let slot = mesh[in.instance_index].material_bind_group_slot;

@if(BINDLESS)
    var output_color: vec4<f32> = color_materials[color_material_indices[slot].material].color;
@else
    var output_color: vec4<f32> = material.color;

@if(VERTEX_COLORS)
    output_color = output_color * in.color;

@if(BINDLESS)
    let uv_transform = color_materials[color_material_indices[slot].material].uv_transform;
@else
    let uv_transform = material.uv_transform;
@if(BINDLESS)
    let flags = color_materials[color_material_indices[slot].material].flags;
@else
    let flags = material.flags;
@if(BINDLESS)
    let alpha_cutoff = color_materials[color_material_indices[slot].material].alpha_cutoff;
@else
    let alpha_cutoff = material.alpha_cutoff;

    let uv = (uv_transform * vec3(in.uv, 1.0)).xy;

    if ((flags & COLOR_MATERIAL_FLAGS_TEXTURE_BIT) != 0u) {
        @if(BINDLESS)
        output_color = output_color * textureSample(
            bindless_textures_2d[color_material_indices[slot].color_texture],
            bindless_samplers_filtering[color_material_indices[slot].color_sampler],
            uv
        );
        @else
        output_color = output_color * textureSample(texture, texture_sampler, uv);
    }

    // ---- the weather pass ------------------------------------------------
    // Two slow waves in tile space: a broad drift and a diagonal band. Both are
    // far below the contrast of the texture itself, so the map reads as
    // shifting light rather than as a moving pattern.
    let tiles = weather.data.yz;
    let time = weather.data.x;
    let tile = uv * tiles;
    let drift = sin(tile.x * 0.35 + time * 0.22) * cos(tile.y * 0.27 - time * 0.17);
    let band = sin(dot(tile, vec2<f32>(0.13, 0.21)) + time * 0.35);
    let shimmer = 1.0 + (drift * 0.6 + band * 0.4) * weather.data.w;
    output_color = vec4<f32>(output_color.rgb * shimmer, output_color.a);

    output_color = alpha_discard(flags, alpha_cutoff, output_color);

@if(TONEMAP_IN_SHADER)
    output_color = tonemapping::tone_mapping(output_color, view.color_grading);
@if(SRGB_OUTPUT)
    output_color = vec4(linear_to_srgb(output_color.rgb), output_color.a);
@if(OKLAB_OUTPUT)
    output_color = vec4(linear_rgb_to_oklab(output_color.rgb), output_color.a);
    return output_color;
}

fn alpha_discard(flags: u32, alpha_cutoff: f32, output_color: vec4<f32>) -> vec4<f32> {
    var color = output_color;
    let alpha_mode = flags & COLOR_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == COLOR_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
        color.a = 1.0;
    }
    else {
        @if(MAY_DISCARD)
        if alpha_mode == COLOR_MATERIAL_FLAGS_ALPHA_MODE_MASK {
           if color.a >= alpha_cutoff {
                // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
                color.a = 1.0;
            } else {
                // NOTE: output_color.a < in.material.alpha_cutoff should not be rendered
                discard;
            }
        }
    }

    return color;
}
