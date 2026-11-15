//! Paints the effect slots of live backgrounds (LIVEBG1), in the projector and in any window
//! that shows previews of one.
//!
//! Before Rune draws a frame, `LivePainter::prepare` finds every live slot among the frame's
//! instances: six kinds become `rune-fx` fills, prepared in one call, and the stars and the
//! kaleidoscope are Artosia's own shader (`live.wgsl`). Rune's `draw_with` then calls back at
//! each slot, in painter order, and the painter draws that slot there.

use std::collections::HashMap;

use artosia_slides::live::{self, LiveKind};
use rune::compose::DeviceKey;
use rune::render::{EffectSite, Instance, KIND_EFFECT};
use rune::{Compositor, Frame};
use rune_fx::{
    AuroraParams, Fx, FxFrame, FxLayer, FxOptions, FxRenderer, FxSlot, MeshGradientParams, MetaballParams, Rgb, Shape,
    Stops, SwirlParams, VoronoiParams, WaveParams,
};

const SHADER: &str = include_str!("live.wgsl");
/// One slot's uniform, padded to the 256-byte offset alignment every adapter accepts.
const SLOT_BYTES: u64 = 256;
const UNIFORM_FLOATS: usize = 24;

/// How one live slot is drawn.
#[derive(Clone, Copy)]
enum Paint {
    /// A `rune-fx` layer, by its place in the prepared list.
    Fill(usize),
    /// Artosia's own shader, by the slot's place in the uniform buffer.
    Own(usize),
}

struct Own {
    layout: wgpu::BindGroupLayout,
    module: wgpu::ShaderModule,
    pipelines: HashMap<(wgpu::TextureFormat, bool), wgpu::RenderPipeline>,
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
    slots: u64,
}

impl Own {
    fn new(device: &wgpu::Device, slots: u64) -> Own {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("artosia live"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: wgpu::BufferSize::new(UNIFORM_FLOATS as u64 * 4) },
                count: None,
            }],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("artosia live"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("artosia live"), size: SLOT_BYTES * slots, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("artosia live"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &buffer, offset: 0, size: wgpu::BufferSize::new(UNIFORM_FLOATS as u64 * 4) }) }],
        });
        Own { layout, module, pipelines: HashMap::new(), buffer, group, slots }
    }

    fn pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        let (layout, module) = (&self.layout, &self.module);
        self.pipelines.entry((format, false)).or_insert_with(|| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("artosia live"), bind_group_layouts: &[Some(layout)], immediate_size: 0 });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("artosia live"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState { module, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState { module, entry_point: Some("fs"), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        })
    }
}

struct Gpu {
    key: DeviceKey,
    fx: FxRenderer,
    own: Own,
}

/// What one frame found and prepared.
#[derive(Default)]
struct Found {
    /// Live slots by their place in the instance list.
    by_index: HashMap<u32, Paint>,
    /// The same, by mark and data, for a slot whose painter index differs from its place.
    by_slot: Vec<(u32, [u32; 4], Paint)>,
    /// Own-shader uniforms, by slot, written at each slot's draw (its pass decides the origin).
    own: Vec<[f32; UNIFORM_FLOATS]>,
}

/// Finds and paints live background slots. Keep one per window.
#[derive(Default)]
pub struct LivePainter {
    gpu: Option<Gpu>,
    found: Found,
    moving: bool,
}

fn colour(c: [u8; 3]) -> Rgb {
    rune_fx::rgb((c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
}

fn between(a: [u8; 3], b: [u8; 3], k: f32) -> [u8; 3] {
    [0, 1, 2].map(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8)
}

/// The `rune-fx` fill for a kind, sized for a slot `h` points tall; None for the kinds Artosia
/// draws itself.
fn fill(kind: LiveKind, [dark, middle, light]: [[u8; 3]; 3], h: f32) -> Option<Fx> {
    let unit = h / 1080.0;
    let fx = match kind {
        LiveKind::Flow => Fx::fill(MeshGradientParams {
            colors: Stops::new(&[colour(dark), colour(middle), colour(light), colour(between(dark, middle, 0.5)), colour(between(middle, light, 0.4))]),
            ..MeshGradientParams::dusk()
        }),
        LiveKind::Aurora => Fx::aurora(AuroraParams { a: colour(middle), b: colour(light), c: colour(between(middle, light, 0.5)), scale: 260.0 * unit, intensity: 0.7, feather: 90.0 * unit, ..AuroraParams::default() }),
        LiveKind::Ink => Fx::fill(SwirlParams { colors: Stops::new(&[colour(dark), colour(middle), colour(light), colour(dark)]), scale: 420.0 * unit, ..SwirlParams::ink() }),
        LiveKind::Sea => Fx::fill(WaveParams {
            colors: Stops::new(&[colour(light), colour(between(middle, light, 0.5)), colour(middle), colour(between(dark, middle, 0.5)), colour(dark)]),
            ..WaveParams::sea()
        }),
        LiveKind::Glass => Fx::fill(VoronoiParams {
            colors: Stops::new(&[colour(dark), colour(middle), colour(light), colour(between(middle, light, 0.5)), colour(between(dark, light, 0.5))]),
            edge_color: colour(between([0, 0, 0], dark, 0.4)),
            scale: 150.0 * unit,
            edge: 6.0 * unit,
            ..VoronoiParams::stained()
        }),
        LiveKind::Lava => Fx::fill(MetaballParams { colors: Stops::new(&[colour(middle), colour(light), colour(between(middle, light, 0.5))]), base: colour(dark), ..MetaballParams::lava() }),
        LiveKind::Stars | LiveKind::Kaleidoscope => return None,
    };
    Some(fx)
}

fn effect_mark(i: &Instance) -> u32 {
    (i.border[0] as u32 & 0xFFFF) | (i.border[1] as u32 & 0xFFFF) << 16
}

impl LivePainter {
    pub fn new() -> LivePainter {
        LivePainter::default()
    }

    /// True while a live background on screen moves: the window keeps drawing frames.
    pub fn moving(&self) -> bool {
        self.moving
    }

    /// Finds this frame's live slots and prepares them. False when there are none, so the
    /// caller can draw the frame the plain way.
    pub fn prepare<S: 'static>(&mut self, f: &mut Frame<'_, S>) -> bool {
        let t = f.rt.t as f32;
        self.found = Found::default();
        self.moving = false;
        let mut layers: Vec<FxLayer> = Vec::new();
        let mut slots: Vec<FxSlot> = Vec::new();
        let mut scale = 1.0f32;
        for (index, instance) in f.rt.instances.iter().enumerate() {
            if instance.params[1] != KIND_EFFECT {
                continue;
            }
            let mark = effect_mark(instance);
            let Some((kind, colours, pace)) = live::read(mark, instance.uv) else { continue };
            scale = instance.params[3].max(0.01);
            let [x, y, w, h] = instance.rect.map(|v| v / scale);
            self.moving |= pace > 0.0;
            let paint = match fill(kind, colours, h) {
                Some(fx) => {
                    let mut layer = fx.on(Shape::rounded(x, y, w, h, 0.0));
                    layer.time = t * pace;
                    layer.reduced_motion = pace == 0.0;
                    layer.seed = mark;
                    layers.push(layer);
                    slots.push(FxSlot::default().scale(scale).clip(instance.clip, instance.fade));
                    Paint::Fill(layers.len() - 1)
                }
                None => {
                    let k = if kind == LiveKind::Stars { 0.0 } else { 1.0 };
                    let mut u = [0.0f32; UNIFORM_FLOATS];
                    u[0..4].copy_from_slice(&instance.rect);
                    // [4], [5]: the pass's origin, filled in at the slot's draw.
                    u[6] = t * pace;
                    u[7] = k;
                    for (n, c) in colours.iter().enumerate() {
                        u[8 + n * 4..11 + n * 4].copy_from_slice(&c.map(|v| v as f32 / 255.0));
                        u[11 + n * 4] = 1.0;
                    }
                    u[20] = if f.srgb_blend() { 1.0 } else { 0.0 };
                    self.found.own.push(u);
                    Paint::Own(self.found.own.len() - 1)
                }
            };
            self.found.by_index.insert(index as u32, paint);
            self.found.by_slot.push((mark, instance.uv.map(f32::to_bits), paint));
        }
        if self.found.by_index.is_empty() {
            return false;
        }
        let key = f.device_key();
        let wanted = (self.found.own.len() as u64).max(4);
        if self.gpu.as_ref().is_none_or(|g| g.key != key || g.own.slots < wanted) {
            let mut fx = FxRenderer::new(f.device, f.format, FxOptions { dither: true });
            fx.add_format(f.device, wgpu::TextureFormat::Rgba16Float);
            self.gpu = Some(Gpu { key, fx, own: Own::new(f.device, wanted.next_power_of_two()) });
        }
        let Some(gpu) = self.gpu.as_mut() else { return false };
        gpu.fx.add_format(f.device, f.format);
        gpu.fx.set_srgb_blend(f.srgb_blend());
        let frame = FxFrame { viewport: [f.size.0, f.size.1], scale, layers: &layers, backdrop: None, sources: &[] };
        gpu.fx.prepare_with_slots(f.device, f.queue, &frame, &slots);
        true
    }

    /// Draws Rune's frame into `target`, painting each live slot where it stands.
    pub fn draw<S: 'static>(&mut self, f: &mut Frame<'_, S>, target: &wgpu::TextureView) {
        let clear = f.rt.clear_color();
        f.ui.prepare(f.device, f.queue, &mut f.rt.text.atlas, &f.rt.instances, f.size);
        let Some(gpu) = self.gpu.as_mut() else {
            f.ui.draw(f.encoder, target, clear);
            return;
        };
        let found = &self.found;
        let (device, queue) = (f.device, f.queue);
        let Gpu { fx, own, .. } = gpu;
        f.ui.draw_with(f.encoder, target, clear, &mut |pass: &mut wgpu::RenderPass<'_>, site: &EffectSite<'_>| {
            let paint = found.by_index.get(&site.index).copied().filter(|_| live::read(site.mark, site.data).is_some()).or_else(|| {
                let data = site.data.map(f32::to_bits);
                found.by_slot.iter().find(|(mark, d, _)| *mark == site.mark && *d == data).map(|(_, _, paint)| *paint)
            });
            match paint {
                Some(Paint::Fill(layer)) => fx.draw_layer_in(pass, site.format, site.origin, layer),
                Some(Paint::Own(at)) => {
                    let Some(mut u) = found.own.get(at).copied() else { return };
                    u[4] = site.origin[0];
                    u[5] = site.origin[1];
                    u[20] = if site.srgb_blend { 1.0 } else { 0.0 };
                    let bytes: Vec<u8> = u.iter().flat_map(|v| v.to_ne_bytes()).collect();
                    let offset = at as u64 * SLOT_BYTES;
                    queue.write_buffer(&own.buffer, offset, &bytes);
                    let pipeline = own.pipeline(device, site.format).clone();
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &own.group, &[offset as u32]);
                    pass.draw(0..3, 0..1);
                }
                None => {}
            }
        });
    }
}

/// A window's compositor that only paints live backgrounds: the operator window, whose
/// previews, samples and tiles may show one.
#[derive(Default)]
pub struct LiveCompositor {
    painter: LivePainter,
}

impl LiveCompositor {
    pub fn new() -> LiveCompositor {
        LiveCompositor::default()
    }
}

impl<S: 'static> Compositor<S> for LiveCompositor {
    fn render(&mut self, f: &mut Frame<'_, S>) {
        if !self.painter.prepare(f) {
            f.draw_ui();
            return;
        }
        let target = f.target;
        self.painter.draw(f, target);
    }

    fn animating(&self) -> bool {
        self.painter.moving()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_kinds_are_fills_and_two_are_artosias_own() {
        let colours = live::COLOUR_SETS[0].1;
        let own: Vec<LiveKind> = LiveKind::ALL.into_iter().filter(|kind| fill(*kind, colours, 1080.0).is_none()).collect();
        assert_eq!(own, [LiveKind::Stars, LiveKind::Kaleidoscope]);
    }
}
