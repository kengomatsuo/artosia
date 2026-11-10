// Draws the finished UI texture through a homography and a colour matrix.
// One full-screen triangle; each pixel maps back into the source by the inverse matrix.

struct Uniforms {
    m0: vec4<f32>,    // inverse homography rows: screen fraction -> unit square
    m1: vec4<f32>,
    m2: vec4<f32>,
    grade: vec4<f32>, // brightness, contrast, saturation, grading on (1) or off (0)
    flags: vec4<f32>, // source holds linear values (1) or sRGB-encoded (0), bicubic (1)
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
}

fn encode(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn decode(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn tap(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(src, smp, uv, 0.0);
}

// Catmull-Rom through nine bilinear taps.
fn bicubic(uv: vec2<f32>, dims: vec2<f32>) -> vec4<f32> {
    let pos = uv * dims;
    let c = floor(pos - 0.5) + 0.5;
    let f = pos - c;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let t0 = (c - 1.0) / dims;
    let t12 = (c + w2 / w12) / dims;
    let t3 = (c + 2.0) / dims;
    var r = tap(vec2<f32>(t0.x, t0.y)) * (w0.x * w0.y);
    r += tap(vec2<f32>(t12.x, t0.y)) * (w12.x * w0.y);
    r += tap(vec2<f32>(t3.x, t0.y)) * (w3.x * w0.y);
    r += tap(vec2<f32>(t0.x, t12.y)) * (w0.x * w12.y);
    r += tap(vec2<f32>(t12.x, t12.y)) * (w12.x * w12.y);
    r += tap(vec2<f32>(t3.x, t12.y)) * (w3.x * w12.y);
    r += tap(vec2<f32>(t0.x, t3.y)) * (w0.x * w3.y);
    r += tap(vec2<f32>(t12.x, t3.y)) * (w12.x * w3.y);
    r += tap(vec2<f32>(t3.x, t3.y)) * (w3.x * w3.y);
    let a = clamp(r.a, 0.0, 1.0);
    return vec4<f32>(clamp(r.rgb, vec3<f32>(0.0), vec3<f32>(a)), a);
}

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(src));
    let s = vec3<f32>(pos.xy / dims, 1.0);
    let q = vec3<f32>(dot(u.m0.xyz, s), dot(u.m1.xyz, s), dot(u.m2.xyz, s));
    let uv = q.xy / max(q.z, 1e-6);
    // Edge coverage from the pixel's footprint in the source, so the quad's edge is smooth.
    let w = max(fwidth(uv), vec2<f32>(1e-6));
    let cx = clamp(uv.x / w.x + 0.5, 0.0, 1.0) * clamp((1.0 - uv.x) / w.x + 0.5, 0.0, 1.0);
    let cy = clamp(uv.y / w.y + 0.5, 0.0, 1.0) * clamp((1.0 - uv.y) / w.y + 0.5, 0.0, 1.0);
    let cover = select(0.0, cx * cy, q.z > 1e-6);
    if cover <= 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let at = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0));
    var c = tap(at);
    if u.flags.y > 0.5 {
        c = bicubic(at, dims);
    }
    var rgb = c.rgb;
    if u.grade.w > 0.5 {
        // Grade in display (encoded) space, on straight colour.
        var d = rgb / max(c.a, 1e-5);
        if u.flags.x > 0.5 {
            d = encode(d);
        }
        let luma = dot(d, vec3<f32>(0.2126, 0.7152, 0.0722));
        d = mix(vec3<f32>(luma), d, u.grade.z);
        d = (d - 0.5) * u.grade.y + 0.5 + u.grade.x;
        d = clamp(d, vec3<f32>(0.0), vec3<f32>(1.0));
        if u.flags.x > 0.5 {
            d = decode(d);
        }
        rgb = d * c.a;
    }
    return vec4<f32>(rgb * cover, 1.0);
}
