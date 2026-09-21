// Hex lines drawn from the mesh's own edge data, plus an optional tint by detail level.
// UV0 = (edge_w, border_level): edge_w is 0 at a cell's centre and 1 at its corners.
// UV1 = (cell_level, 0): 0 shaku, 1 ken, 2 cho, 3 ri.

#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing}

// The material bind group is @group(3), not @group(2), on this Bevy build (0.19.1):
// bevy_pbr::material::MATERIAL_BIND_GROUP_INDEX = 3, because group 2 is reserved for
// GPU mesh-preprocessing data. Verified by running `examples/shader_check.rs` and reading
// wgpu's validation error, which named the exact missing (group, binding) pair.
@group(3) @binding(100) var<uniform> line_mode: u32;
@group(3) @binding(101) var<uniform> tint_by_level: u32;

fn level_colour(level: f32) -> vec4<f32> {
    if level < 0.5 { return vec4<f32>(0.10, 0.10, 0.12, 1.0); }   // shaku: thin and dark
    if level < 1.5 { return vec4<f32>(0.85, 0.87, 0.90, 1.0); }   // ken: light
    if level < 2.5 { return vec4<f32>(0.90, 0.80, 0.25, 1.0); }   // cho: yellow
    return vec4<f32>(0.85, 0.25, 0.20, 1.0);                      // ri: red
}

fn level_tint(level: f32) -> vec4<f32> {
    if level < 0.5 { return vec4<f32>(1.00, 1.00, 1.00, 1.0); }
    if level < 1.5 { return vec4<f32>(0.80, 0.90, 1.00, 1.0); }
    if level < 2.5 { return vec4<f32>(1.00, 0.90, 0.75, 1.0); }
    return vec4<f32>(1.00, 0.75, 0.75, 1.0);
}

fn line_width_px(border_level: f32) -> f32 {
    return 1.0 + border_level * 0.8;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let edge_w = in.uv.x;
    let border_level = in.uv.y;
    let cell_level = in.uv_b.x;

    if tint_by_level == 1u {
        pbr_input.material.base_color = pbr_input.material.base_color * level_tint(cell_level);
    }

    // Only top faces carry edge data; side faces have edge_w == 0 everywhere.
    var draw_line = false;
    if line_mode == 1u {
        draw_line = cell_level < 0.5;              // the borders of loaded shaku only
    } else if line_mode == 2u {
        draw_line = true;                          // every level, nested
    }

    if draw_line {
        let distance_px = (1.0 - edge_w) / max(fwidth(edge_w), 1e-6);
        let width = line_width_px(border_level);
        let strength = 1.0 - smoothstep(width - 1.0, width, distance_px);
        pbr_input.material.base_color = mix(
            pbr_input.material.base_color,
            level_colour(border_level),
            strength,
        );
    }

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
