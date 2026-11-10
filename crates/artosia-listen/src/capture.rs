//! Audio capture through cpal (WASAPI on Windows, CoreAudio on macOS, ALSA on Linux).

use std::str::FromStr;
use std::sync::mpsc::SyncSender;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{DeviceId, ErrorKind, FromSample, SampleFormat, SizedSample, Stream, StreamConfig};

use crate::audio::mix_to_mono;
use crate::{Device, ListenError};

/// What the capture callback sends to the worker.
pub enum Capture {
    /// Mono samples at the device's own rate.
    Audio(Vec<f32>),
    /// A stream error worth showing.
    Error(String),
}

/// Microphones and line inputs on the default host.
pub fn input_devices() -> Vec<Device> {
    let host = cpal::default_host();
    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|d| {
            let id = d.id().ok()?.to_string();
            let name = d.description().map(|desc| desc.name().to_string()).unwrap_or_else(|_| id.clone());
            Some(Device { id, name })
        })
        .collect()
}

/// A running capture; dropping it stops the stream. Not `Send`: keep it on the thread that made it.
pub struct Running {
    _stream: Stream,
    pub sample_rate: u32,
}

pub fn open(device: Option<&str>, tx: SyncSender<Capture>) -> Result<Running, ListenError> {
    let host = cpal::default_host();
    let device = match device {
        None => host.default_input_device().ok_or(ListenError::NoDevice("no default input device".into()))?,
        Some(id) => {
            let parsed = DeviceId::from_str(id).map_err(|e| ListenError::NoDevice(format!("bad device id {id}: {e}")))?;
            host.device_by_id(&parsed).ok_or_else(|| ListenError::NoDevice(format!("device {id} is not connected")))?
        }
    };
    let supported = device.default_input_config().map_err(|e| ListenError::Stream(e.to_string()))?;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let channels = usize::from(config.channels.max(1));
    let sample_rate = config.sample_rate;

    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, &config, channels, tx),
        SampleFormat::I16 => build::<i16>(&device, &config, channels, tx),
        SampleFormat::I32 => build::<i32>(&device, &config, channels, tx),
        SampleFormat::U16 => build::<u16>(&device, &config, channels, tx),
        SampleFormat::I8 => build::<i8>(&device, &config, channels, tx),
        SampleFormat::U8 => build::<u8>(&device, &config, channels, tx),
        other => return Err(ListenError::Stream(format!("unsupported sample format {other}"))),
    }?;
    stream.play().map_err(|e| ListenError::Stream(e.to_string()))?;
    Ok(Running { _stream: stream, sample_rate })
}

fn build<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    tx: SyncSender<Capture>,
) -> Result<Stream, ListenError>
where
    T: SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    let data_tx = tx.clone();
    let mut scratch: Vec<f32> = Vec::new();
    device
        .build_input_stream(
            *config,
            move |data: &[T], _| {
                scratch.clear();
                scratch.extend(data.iter().map(|s| f32::from_sample_(*s)));
                let mut mono = Vec::with_capacity(scratch.len() / channels + 1);
                mix_to_mono(&scratch, channels, &mut mono);
                // A full queue means the worker is behind: dropping a block beats growing without bound.
                let _ = data_tx.try_send(Capture::Audio(mono));
            },
            move |err| {
                if !matches!(err.kind(), ErrorKind::Xrun) {
                    let _ = tx.try_send(Capture::Error(err.to_string()));
                }
            },
            None,
        )
        .map_err(|e| ListenError::Stream(e.to_string()))
}
