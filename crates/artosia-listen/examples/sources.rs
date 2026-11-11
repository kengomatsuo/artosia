//! Proves that audio sources open and deliver sound, through the code the listener uses.
//!
//! cargo run -p artosia-listen --example sources -- list
//! cargo run -p artosia-listen --example sources -- mic [name-or-id|default]   3 seconds
//! cargo run -p artosia-listen --example sources -- tone                       a quiet tone through the default
//!                                                                             output, read back by loopback
//! cargo run -p artosia-listen --example sources -- tempo <bpm>                10 s of a drum pattern through the
//!                                                                             default output, read back by loopback
//!
//! Levels and tempo only: no audio is kept, and every sound played is short and quiet.

use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use artosia_listen::{Probe, SYSTEM_AUDIO, TempoTracker, devices, probe};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

fn show(label: &str, result: Result<Probe, artosia_listen::ListenError>) {
    match result {
        Ok(p) => println!(
            "{label}: opened, {} Hz, {} channel(s), {}{}, {} mono frames, peak {:.5} ({:.1} dBFS), rms {:.5} ({:.1} dBFS)",
            p.sample_rate,
            p.channels,
            p.sample_format,
            if p.loopback { ", loopback" } else { "" },
            p.frames,
            p.peak,
            db(p.peak),
            p.rms,
            db(p.rms),
        ),
        Err(e) => println!("{label}: ERROR {e}"),
    }
}

fn db(v: f32) -> f32 {
    if v <= 0.0 { f32::NEG_INFINITY } else { 20.0 * v.log10() }
}

/// Plays `render(seconds_since_start, sample_rate) -> sample` on every channel of the default
/// output until the returned stream is dropped.
fn play(render: impl Fn(f32, f32) -> f32 + Send + 'static) -> cpal::Stream {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("a default output device");
    let supported = device.default_output_config().expect("an output format");
    let (rate, channels, format) = (supported.sample_rate() as f32, usize::from(supported.channels()), supported.sample_format());
    let config: cpal::StreamConfig = supported.into();
    fn build<T: SizedSample + FromSample<f32> + Send + 'static>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        rate: f32,
        channels: usize,
        render: impl Fn(f32, f32) -> f32 + Send + 'static,
    ) -> cpal::Stream {
        let mut frame = 0u64;
        device
            .build_output_stream(
                *config,
                move |data: &mut [T], _| {
                    for chunk in data.chunks_mut(channels) {
                        let v = T::from_sample_(render(frame as f32 / rate, rate));
                        chunk.iter_mut().for_each(|s| *s = v);
                        frame += 1;
                    }
                },
                |e| eprintln!("output error: {e}"),
                None,
            )
            .expect("an output stream")
    }
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, &config, rate, channels, render),
        SampleFormat::I16 => build::<i16>(&device, &config, rate, channels, render),
        SampleFormat::I32 => build::<i32>(&device, &config, rate, channels, render),
        other => panic!("output format {other} is not handled by this example"),
    };
    stream.play().expect("output plays");
    stream
}

/// A kick, a snare on the backbeat and eighth-note hats, at `bpm`, quiet.
fn drums(t: f32, bpm: f32) -> f32 {
    let beat = 60.0 / bpm;
    let in_beat = t % beat;
    let index = (t / beat) as u64;
    let kick = if index.is_multiple_of(2) { (2.0 * PI * 70.0 * in_beat).sin() * (-in_beat * 25.0).exp() } else { 0.0 };
    let snare = if index % 2 == 1 { noise(t) * (-in_beat * 30.0).exp() * 0.6 } else { 0.0 };
    let off = t % (beat / 2.0);
    let hat = noise(t * 1.7) * (-off * 90.0).exp() * 0.25;
    0.15 * (kick + snare + hat)
}

fn noise(t: f32) -> f32 {
    let x = (t * 48_000.0).floor();
    let v = (x * 12.9898).sin() * 43758.547;
    (v - v.floor()) * 2.0 - 1.0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("list") | None => {
            println!("Inputs:");
            for d in devices() {
                println!("  {}  [{}]", d.name, d.id);
            }
            let host = cpal::default_host();
            let name = |d: Option<cpal::Device>| d.and_then(|d| d.description().ok().map(|x| x.name().to_string()));
            println!("Default input:  {:?}", name(host.default_input_device()));
            println!("Default output: {:?}", name(host.default_output_device()));
        }
        Some("mic") => {
            let wanted = args.get(1).map_or("default", String::as_str);
            let found = devices();
            let id = if wanted == "default" {
                None
            } else if wanted == SYSTEM_AUDIO {
                Some(SYSTEM_AUDIO.to_string())
            } else {
                let lower = wanted.to_lowercase();
                let hit = found.iter().find(|d| d.id == wanted || d.name.to_lowercase().contains(&lower));
                match hit {
                    Some(d) => {
                        println!("{wanted} is {} [{}]", d.name, d.id);
                        Some(d.id.clone())
                    }
                    None => {
                        println!("{wanted}: ERROR no input device matches");
                        return;
                    }
                }
            };
            show(wanted, probe(id.as_deref(), Duration::from_secs(3), |_, _| {}));
            // The name alone, as an older saved setting held it.
            if let Some(d) = found.iter().find(|d| Some(&d.id) == id.as_ref()) {
                show(&format!("{} by name", d.name), probe(Some(&d.name), Duration::from_secs(1), |_, _| {}));
            }
        }
        Some("tone") => {
            let _out = play(|t, _| if t < 2.6 { 0.05 * (2.0 * PI * 440.0 * t).sin() } else { 0.0 });
            std::thread::sleep(Duration::from_millis(300));
            show("system audio, 440 Hz tone at -26 dBFS", probe(Some(SYSTEM_AUDIO), Duration::from_millis(2000), |_, _| {}));
        }
        Some("tempo") => {
            let bpm: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(120.0);
            let _out = play(move |t, _| drums(t, bpm));
            std::thread::sleep(Duration::from_millis(300));
            let tracker: Arc<Mutex<Option<TempoTracker>>> = Arc::new(Mutex::new(None));
            let mut fed = 0usize;
            let mut first_stable: Option<f32> = None;
            let mut reads: Vec<(f32, f32, f32)> = Vec::new();
            let result = probe(Some(SYSTEM_AUDIO), Duration::from_secs(10), |block, rate| {
                let mut slot = tracker.lock().unwrap();
                let tracker = slot.get_or_insert_with(|| TempoTracker::new(rate));
                fed += block.len();
                let now = fed as f32 / rate as f32;
                tracker.push(block, |t| {
                    if let Some(t) = t {
                        reads.push((now, t.bpm, t.confidence));
                        if first_stable.is_none() && (t.bpm - bpm).abs() <= 2.0 {
                            first_stable = Some(now);
                        }
                    }
                });
            });
            show(&format!("system audio, drums at {bpm} BPM"), result);
            println!("tempo reports: {}", reads.len());
            for (at, b, c) in reads.iter().step_by(2) {
                println!("  {at:5.1} s  {b:6.1} BPM  confidence {c:.2}");
            }
            println!("first reading within 2 BPM of {bpm}: {first_stable:?}");
        }
        Some(other) => println!("unknown command {other}"),
    }
}
