// Artosia's own live backgrounds (LIVEBG1): a star field and a kaleidoscope, each a few
// arithmetic steps per pixel. One triangle covers the pass; the slot's rectangle, the scissor
// Rune sets and `discard` keep it inside the slot.

struct Slot {
    // x, y, width, height of the slot in window pixels.
    rect: vec4<f32>,
    // The pass's top-left pixel in window pixels, then time in seconds, then the kind
    // (0 stars, 1 kaleidoscope).
    origin_time_kind: vec4<f32>,
    // Dark, middle and light, as sRGB 0..1.
    dark: vec4<f32>,
    middle: vec4<f32>,
    light: vec4<f32>,
    // 1 when the frame blends in sRGB: the output is encoded here.
    flags: vec4<f32>,
};

@group(0) @binding(0) var<uniform> slot: Slot;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3<f32>(0.0031308));
}

fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var q = p;
    for (var k = 0; k < 4; k = k + 1) {
        v = v + amp * noise(q);
        q = q * 2.03 + vec2<f32>(17.1, 9.2);
        amp = amp * 0.5;
    }
    return v;
}

// One layer of stars: a star per cell at a hashed place, twinkling at its own phase.
fn stars(p: vec2<f32>, cells: f32, size: f32, t: f32, seed: f32) -> f32 {
    let g = p * cells;
    let cell = floor(g);
    let h = hash2(cell + seed);
    if (h < 0.55) {
        return 0.0;
    }
    let at = vec2<f32>(hash2(cell + seed + 3.1), hash2(cell + seed + 7.7)) * 0.8 + 0.1;
    let d = length(fract(g) - at) / cells;
    let twinkle = 0.55 + 0.45 * sin(t * (0.8 + h * 1.6) + h * 40.0);
    let core = smoothstep(size, 0.0, d);
    return core * core * twinkle * (h - 0.55) / 0.45;
}

@fragment
fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let p = at.xy + slot.origin_time_kind.xy;
    let uv = (p - slot.rect.xy) / slot.rect.zw;
    if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) {
        discard;
    }
    let t = slot.origin_time_kind.z;
    let aspect = slot.rect.z / max(slot.rect.w, 1.0);
    let dark = to_linear(slot.dark.rgb);
    let middle = to_linear(slot.middle.rgb);
    let light = to_linear(slot.light.rgb);
    var colour: vec3<f32>;
    if (slot.origin_time_kind.w < 0.5) {
        // Stars: a night sky from dark at the top to the middle colour at the foot, a faint
        // band of haze, and three layers of stars that drift very slowly.
        let sky = mix(dark, middle, smoothstep(0.0, 1.3, uv.y));
        let q = vec2<f32>(uv.x * aspect, uv.y) + vec2<f32>(t * 0.004, 0.0);
        let haze = fbm(q * 2.5 + vec2<f32>(t * 0.01, 0.0)) * smoothstep(0.9, 0.2, abs(uv.y - 0.45 - 0.15 * sin(uv.x * 2.0)));
        let shine = stars(q, 90.0, 0.0035, t, 1.0) + stars(q, 45.0, 0.005, t, 11.0) * 0.9 + stars(q, 20.0, 0.007, t, 23.0);
        colour = sky + middle * haze * 0.18 + light * shine * 1.4;
    } else {
        // Kaleidoscope: five drifting circles, folded into twelve mirrored wedges round the
        // centre that turn slowly. Where circles overlap the glass changes colour; their rims
        // are drawn in the light colour, as the edges of the pieces in a real one.
        var c = uv - vec2<f32>(0.5, 0.5);
        c.x = c.x * aspect;
        let r = length(c);
        let wedge = 6.2831853 / 12.0;
        var a = atan2(c.y, c.x) + t * 0.03;
        a = abs(a - wedge * floor(a / wedge) - wedge * 0.5);
        let q = vec2<f32>(cos(a), sin(a)) * r;
        var inside = 0.0;
        var rim = 0.0;
        for (var i = 0; i < 5; i = i + 1) {
            let f = f32(i);
            let centre = vec2<f32>(0.08 + 0.09 * f, 0.03 + 0.02 * f) + 0.06 * vec2<f32>(sin(t * 0.21 + f * 1.9), cos(t * 0.17 + f * 2.7));
            let radius = 0.07 + 0.03 * sin(t * 0.13 + f * 1.3) + 0.012 * f;
            let d = length(q - centre) - radius;
            inside = inside + step(d, 0.0);
            rim = max(rim, smoothstep(0.006, 0.0, abs(d)));
        }
        // The count of circles a point is inside picks its piece of glass.
        let piece = inside - 3.0 * floor(inside / 3.0);
        var glass = dark;
        if (piece > 0.5 && piece < 1.5) {
            glass = middle;
        } else if (piece > 1.5) {
            glass = mix(middle, light, 0.55);
        }
        // A little grain inside each piece, so the glass is not flat.
        glass = glass * (0.85 + 0.3 * noise(q * 40.0 + inside * 7.0));
        colour = mix(glass, light, rim * 0.9);
        // Darker toward the edges, so words keep their contrast where they sit.
        colour = colour * mix(1.0, 0.6, smoothstep(0.25, 0.8, r));
    }
    if (slot.flags.x > 0.5) {
        colour = to_srgb(clamp(colour, vec3<f32>(0.0), vec3<f32>(1.0)));
    }
    return vec4<f32>(colour, 1.0);
}
