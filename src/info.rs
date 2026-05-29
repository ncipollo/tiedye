const INFO: &str = r#"# tiedye

GPU shader tool that applies WGSL shaders to generate or transform images.

## Usage

```
tiedye shader.wgsl [IMAGE...]                    # renders to output.png
tiedye shader.wgsl [IMAGE...] > out.png          # pipes PNG bytes to stdout
tiedye shader.wgsl [IMAGE...] --output path.png  # explicit output path
```

## Input

- **Images** (optional): one or more image files (PNG, JPEG, etc.) passed as positional arguments.
  - No images: shader renders at 512×512.
  - With images: output dimensions match the first image.

## WGSL Basics

Shaders are written in [WGSL](https://www.w3.org/TR/WGSL/) and run on the GPU in two stages:

- **Vertex** (`@vertex`): positions geometry. Use a full-screen triangle for image effects.
- **Fragment** (`@fragment`): computes the RGBA color of each pixel.

### Uniforms

```wgsl
struct Params { width: u32, height: u32 }
@group(0) @binding(0) var<uniform> params: Params;
```

### Useful built-ins

| Expression             | Description                        |
|------------------------|------------------------------------|
| `@builtin(position)`   | fragment pixel coords (`vec4<f32>`) |
| `atan2(y, x)`          | angle in radians                   |
| `length(v)`            | vector magnitude                   |
| `fract(x)`             | fractional part (0.0–1.0)          |
| `sin(x)`, `cos(x)`     | trigonometric functions            |
| `clamp(x, lo, hi)`     | clamp value to range               |
| `mix(a, b, t)`         | linear interpolation               |

## Bindings Reference

| Group | Binding | Type                   | Description                         |
|-------|---------|------------------------|-------------------------------------|
| 0     | 0       | `uniform Params`       | Output dimensions                   |
| 0     | 1       | `texture_2d<f32>`      | First input image (when provided)   |
| 0     | 2       | `sampler`              | Sampler for the input texture       |

Texture bindings 1 and 2 are only present when at least one image is passed as an argument.

## Simple Example

```wgsl
struct Params { width: u32, height: u32 }
@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0),
    );
    return vec4(pos[i], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pos.xy / vec2(f32(params.width), f32(params.height));
    return vec4(uv.x, uv.y, 0.5, 1.0); // red=x, green=y gradient
}
```

## Input Image Example

Sample from the first input image passed on the command line:

```wgsl
struct Params { width: u32, height: u32 }
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var input_texture: texture_2d<f32>;
@group(0) @binding(2) var input_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0),
    );
    return vec4(pos[i], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pos.xy / vec2(f32(params.width), f32(params.height));
    return textureSample(input_texture, input_sampler, uv);
}
```

Invoke with: `tiedye effect.wgsl photo.png --output result.png`

## Output

| Scenario              | Behavior                              |
|-----------------------|---------------------------------------|
| stdout is a pipe      | PNG bytes written to stdout           |
| stdout is a terminal  | saves to `output.png` in current dir  |
| `--output <path>`     | saves to specified path               |

Output is always a PNG at the shader's render resolution.
"#;

pub fn print_info() {
    print!("{INFO}");
}
