//! The compositor: Rune draws the whole UI into an offscreen texture, then one more pass draws
//! that texture to the window through the corner pin and the colour correction.
//!
//! With the identity setting nothing extra runs: the frame is Rune's own `draw_ui`.

use artosia_slides::frame::Warp;
use rune::compose::DeviceKey;
use rune::{Compositor, Frame};

use crate::handle::WarpHandle;
use crate::homography::{self, Matrix};

const SHADER: &str = include_str!("warp.wgsl");
const UNIFORM_BYTES: u64 = 80;

/// How the pinned picture is resampled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    /// One bilinear tap per pixel: cheapest, softens text under a strong keystone.
    #[default]
    Bilinear,
    /// Catmull-Rom through nine bilinear taps: sharper text, about nine times the sampling.
    Bicubic,
}

/// What the shader needs for one setting.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Prepared {
    inverse: Matrix,
    grade: [f32; 4],
}

fn prepare(warp: &Warp) -> Prepared {
    let m = homography::homography(warp.corners);
    let neutral = warp.brightness == 0.0 && warp.contrast == 1.0 && warp.saturation == 1.0;
    Prepared {
        inverse: homography::inverse(&m),
        grade: [
            warp.brightness.clamp(-1.0, 1.0),
            warp.contrast.clamp(0.0, 4.0),
            warp.saturation.clamp(0.0, 4.0),
            if neutral { 0.0 } else { 1.0 },
        ],
    }
}

/// True when the frame needs no extra pass: the corners are the screen's own and no colour
/// is changed. A degenerate corner setting counts as identity, as `homography` leaves it.
fn is_identity(warp: &Warp) -> bool {
    warp.is_identity() || (homography::homography(warp.corners) == homography::IDENTITY && prepare(warp).grade[3] == 0.0)
}

struct Gpu {
    key: DeviceKey,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniforms: wgpu::Buffer,
    scene: Option<Scene>,
}

/// The offscreen copy of the UI and the bind group that reads it.
struct Scene {
    size: (u32, u32),
    // Held so the view stays valid.
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
}

impl Gpu {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, key: DeviceKey) -> Gpu {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("artosia warp"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let entry = |binding, visibility, ty| wgpu::BindGroupLayoutEntry { binding, visibility, ty, count: None };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("artosia warp"),
            entries: &[
                entry(
                    0,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                ),
                entry(
                    1,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(2, wgpu::ShaderStages::FRAGMENT, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("artosia warp"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("artosia warp"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("artosia warp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("artosia warp"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Gpu { key, format, pipeline, layout, sampler, uniforms, scene: None }
    }

    /// The offscreen target for `size`, made again when the window resized.
    fn ensure_scene(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if self.scene.as_ref().is_none_or(|s| s.size != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("artosia warp scene"),
                size: wgpu::Extent3d { width: size.0.max(1), height: size.1.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("artosia warp"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.uniforms.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            self.scene = Some(Scene { size, _texture: texture, view, group });
        }
    }
}

/// Draws every frame through the pin set on `handle`. Attach with `App::compositor`.
pub struct WarpCompositor {
    handle: WarpHandle,
    filter: Filter,
    gpu: Option<Gpu>,
    seen: u64,
    prepared: Prepared,
    identity: bool,
}

impl WarpCompositor {
    pub fn new(handle: WarpHandle) -> Self {
        WarpCompositor {
            handle,
            filter: Filter::default(),
            gpu: None,
            seen: 0,
            prepared: prepare(&Warp::default()),
            identity: true,
        }
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    fn refresh(&mut self) {
        let version = self.handle.version();
        if version != self.seen || self.seen == 0 {
            let warp = self.handle.get();
            self.identity = is_identity(&warp);
            self.prepared = prepare(&warp);
            self.seen = version;
        }
    }
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_ne_bytes()).collect()
}

impl<S: 'static> Compositor<S> for WarpCompositor {
    fn render(&mut self, f: &mut Frame<'_, S>) {
        self.refresh();
        if self.identity {
            f.draw_ui();
            return;
        }
        if self.gpu.as_ref().is_none_or(|g| g.key != f.device_key() || g.format != f.format) {
            self.gpu = Some(Gpu::new(f.device, f.format, f.device_key()));
        }
        let Some(gpu) = self.gpu.as_mut() else { return };
        let size = f.size;
        let linear = if f.format.is_srgb() { 1.0 } else { 0.0 };
        let [r0, r1, r2] = self.prepared.inverse;
        let bicubic = if self.filter == Filter::Bicubic { 1.0 } else { 0.0 };
        let data = floats(&[
            r0[0], r0[1], r0[2], 0.0, r1[0], r1[1], r1[2], 0.0, r2[0], r2[1], r2[2], 0.0,
            self.prepared.grade[0], self.prepared.grade[1], self.prepared.grade[2], self.prepared.grade[3],
            linear, bicubic, 0.0, 0.0,
        ]);
        f.write_buffer(&gpu.uniforms, 0, &data);
        gpu.ensure_scene(f.device, size);
        let Some(scene) = gpu.scene.as_ref() else { return };
        // Rune's own frame, into the offscreen texture.
        let clear = f.rt.clear_color();
        f.ui.prepare(f.device, f.queue, &mut f.rt.text.atlas, &f.rt.instances, f.size);
        f.ui.draw(f.encoder, &scene.view, clear);
        // Then the pinned, tinted copy into the window.
        let mut pass = f.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("artosia warp"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: f.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&gpu.pipeline);
        pass.set_bind_group(0, &scene.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_needs_no_pass() {
        assert!(is_identity(&Warp::default()));
        assert!(!is_identity(&Warp { brightness: 0.1, ..Warp::default() }));
        let mut tilted = Warp::default();
        tilted.corners[0] = [0.05, 0.0];
        assert!(!is_identity(&tilted));
    }

    #[test]
    fn a_degenerate_pin_with_no_tint_is_identity() {
        let flat = Warp { corners: [[0.5, 0.5]; 4], ..Warp::default() };
        assert!(is_identity(&flat));
    }
}
