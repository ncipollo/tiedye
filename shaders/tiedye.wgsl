struct Params {
    width: u32,
    height: u32,
}

@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let x = c * (1.0 - abs(fract(h / 60.0) * 2.0 - 1.0));
    let m = v - c;
    var rgb: vec3<f32>;
    if h < 60.0 {
        rgb = vec3<f32>(c, x, 0.0);
    } else if h < 120.0 {
        rgb = vec3<f32>(x, c, 0.0);
    } else if h < 180.0 {
        rgb = vec3<f32>(0.0, c, x);
    } else if h < 240.0 {
        rgb = vec3<f32>(0.0, x, c);
    } else if h < 300.0 {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }
    return rgb + vec3<f32>(m, m, m);
}

@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(f32(params.width), f32(params.height));
    let uv = (frag_pos.xy - size * 0.5) / min(size.x, size.y);

    let pi = 3.14159265358979;
    let angle = atan2(uv.y, uv.x);
    let radius = length(uv);

    // Spiral offset: angle normalized to [0,1] plus radius-based swirl
    let spiral = (angle / (2.0 * pi)) + radius * 6.0;

    // Map spiral to vivid tiedye hues
    let hue = fract(spiral) * 360.0;
    let saturation = clamp(0.7 + 0.3 * sin(radius * 20.0), 0.0, 1.0);
    let value = clamp(0.85 + 0.15 * cos(spiral * 3.0 * 2.0 * pi), 0.0, 1.0);

    let rgb = hsv_to_rgb(hue, saturation, value);
    return vec4<f32>(rgb, 1.0);
}
