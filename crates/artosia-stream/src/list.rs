//! What can be captured: monitors, windows and audio inputs.

use crate::args::Audio;
use crate::encode::AudioSource;
use crate::win;
use gstreamer as gst;
use gstreamer::prelude::*;
use serde_json::{Value, json};

/// A WASAPI audio input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    pub name: String,
    pub actual_name: String,
    pub id: String,
    pub is_default: bool,
    /// True for an output device read as an input: what the PC plays.
    pub loopback: bool,
}

/// The audio inputs GStreamer's device monitor sees. Needs `gst::init` first. Off Windows the
/// monitor is asked for every sound source, and each device's own element says which name
/// opens it (`pulsesrc` has a name there, `osxaudiosrc` a number).
#[cfg(not(windows))]
pub fn audio_devices() -> Vec<AudioDevice> {
    let monitor = gst::DeviceMonitor::new();
    let _ = monitor.add_filter(Some("Audio/Source"), None);
    monitor
        .devices()
        .into_iter()
        .filter_map(|d| {
            let element = d.create_element(None).ok()?;
            let device = element.property_value("device");
            let id = device
                .get::<Option<String>>()
                .ok()
                .flatten()
                .or_else(|| device.get::<i32>().ok().map(|n| n.to_string()))
                .or_else(|| device.get::<u32>().ok().map(|n| n.to_string()))?;
            let props = d.properties();
            let text = |name: &str| props.as_ref().and_then(|p| p.get::<String>(name).ok());
            let flag = |name: &str| props.as_ref().and_then(|p| p.get::<bool>(name).ok()).unwrap_or(false);
            Some(AudioDevice {
                name: d.display_name().to_string(),
                actual_name: text("device.description").unwrap_or_default(),
                id,
                is_default: flag("is-default"),
                // PulseAudio calls an input that carries what an output plays a monitor.
                loopback: text("device.class").as_deref() == Some("monitor"),
            })
        })
        .collect()
}

/// The audio inputs GStreamer's device monitor sees. Needs `gst::init` first.
#[cfg(windows)]
pub fn audio_devices() -> Vec<AudioDevice> {
    // Ask the WASAPI provider alone: a DeviceMonitor loads every provider on the PC, which
    // took five seconds here.
    let devices: Vec<gst::Device> =
        match gst::DeviceProviderFactory::find("wasapi2deviceprovider").and_then(|f| f.get()) {
            Some(provider) => provider.devices().into_iter().collect(),
            None => {
                let monitor = gst::DeviceMonitor::new();
                let _ = monitor.add_filter(Some("Audio/Source"), None);
                monitor.devices().into_iter().collect()
            }
        };
    devices
        .into_iter()
        .filter_map(|d| {
            let props = d.properties()?;
            if props.get::<String>("device.api").ok().as_deref() != Some("wasapi2") {
                return None;
            }
            Some(AudioDevice {
                name: d.display_name().to_string(),
                actual_name: props
                    .get::<String>("device.actual-name")
                    .unwrap_or_default(),
                id: props.get::<String>("device.id").ok()?,
                is_default: props.get::<bool>("device.default").unwrap_or(false),
                loopback: props
                    .get::<bool>("wasapi2.device.loopback")
                    .unwrap_or(false),
            })
        })
        .collect()
}

/// The whole `--list` answer.
pub fn list_json() -> Value {
    let monitors: Vec<Value> = win::monitors()
        .into_iter()
        .map(|m| {
            json!({
                "index": m.index, "x": m.x, "y": m.y, "width": m.width,
                "height": m.height, "primary": m.primary,
            })
        })
        .collect();
    let windows: Vec<Value> = win::windows()
        .into_iter()
        .map(|w| {
            json!({
                "title": w.title, "handle": w.handle, "pid": w.pid, "minimized": w.minimized,
            })
        })
        .collect();
    let audio: Vec<Value> = audio_devices()
        .into_iter()
        .map(|a| {
            json!({
                "name": a.name, "actual_name": a.actual_name, "default": a.is_default,
                "loopback": a.loopback,
            })
        })
        .collect();
    json!({ "type": "list", "monitors": monitors, "windows": windows, "audio_inputs": audio })
}

/// Picks the device for a request. The second value is a sentence for the volunteer when the
/// request could not be met and silence is used instead.
pub fn resolve_audio(choice: &Audio, devices: &[AudioDevice]) -> (AudioSource, Option<String>) {
    match choice {
        Audio::None => (AudioSource::Silence, None),
        Audio::Default => match devices.iter().find(|d| d.is_default && !d.loopback) {
            Some(d) => (
                AudioSource::Device {
                    id: d.id.clone(),
                    loopback: false,
                },
                None,
            ),
            // A Linux sound server may flag no input as the default (WSL's does not) and still
            // have one, which its source opens when no device is named.
            None if !cfg!(windows) && devices.iter().any(|d| !d.loopback) => (AudioSource::SystemDefault, None),
            None => (
                AudioSource::Silence,
                Some("No audio input was found. The stream will be silent.".to_string()),
            ),
        },
        Audio::Device(wanted) => match find_device(devices, wanted) {
            Some(d) => (
                AudioSource::Device {
                    id: d.id.clone(),
                    loopback: d.loopback,
                },
                None,
            ),
            None => (
                AudioSource::Silence,
                Some(format!(
                    "The audio input \"{wanted}\" was not found. The stream will be silent."
                )),
            ),
        },
    }
}

fn find_device<'a>(devices: &'a [AudioDevice], wanted: &str) -> Option<&'a AudioDevice> {
    let lower = wanted.to_lowercase();
    devices
        .iter()
        .find(|d| d.name == wanted || d.actual_name == wanted)
        .or_else(|| {
            devices.iter().find(|d| {
                d.name.to_lowercase().contains(&lower)
                    || d.actual_name.to_lowercase().contains(&lower)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(name: &str, actual: &str, default: bool, loopback: bool) -> AudioDevice {
        AudioDevice {
            name: name.into(),
            actual_name: actual.into(),
            id: format!("id-{name}"),
            is_default: default,
            loopback,
        }
    }

    fn sample() -> Vec<AudioDevice> {
        vec![
            dev(
                "Default Audio Capture Device",
                "Microphone (Razer)",
                true,
                false,
            ),
            dev("Default Audio Render Device", "Speakers", true, true),
            dev("Line In (Realtek)", "Line In (Realtek)", false, false),
        ]
    }

    #[test]
    fn default_is_the_default_input_not_the_speakers() {
        let (src, note) = resolve_audio(&Audio::Default, &sample());
        assert!(note.is_none());
        assert_eq!(
            src,
            AudioSource::Device {
                id: "id-Default Audio Capture Device".into(),
                loopback: false
            }
        );
    }

    #[test]
    fn with_no_input_flagged_the_systems_own_default_is_opened_off_windows() {
        let unflagged = vec![dev("RDP Source", "RDP Source", false, false), dev("Monitor of RDP Sink", "Monitor of RDP Sink", false, true)];
        let (src, note) = resolve_audio(&Audio::Default, &unflagged);
        if cfg!(windows) {
            assert_eq!(src, AudioSource::Silence);
            assert!(note.is_some());
        } else {
            assert_eq!(src, AudioSource::SystemDefault);
            assert!(note.is_none());
        }
        // What plays out of the speakers is never taken for the microphone.
        let (src, _) = resolve_audio(&Audio::Default, &unflagged[1..]);
        assert_eq!(src, AudioSource::Silence);
    }

    #[test]
    fn a_name_matches_exactly_then_by_part() {
        let (src, _) = resolve_audio(&Audio::Device("line in".into()), &sample());
        assert!(matches!(src, AudioSource::Device { .. }));
        let (src, _) = resolve_audio(&Audio::Device("Microphone (Razer)".into()), &sample());
        assert!(matches!(src, AudioSource::Device { .. }));
    }

    #[test]
    fn a_missing_device_gives_silence_and_a_sentence() {
        let (src, note) = resolve_audio(&Audio::Device("Mixer".into()), &sample());
        assert_eq!(src, AudioSource::Silence);
        assert!(note.unwrap().contains("Mixer"));
        let (src, note) = resolve_audio(&Audio::Default, &[]);
        assert_eq!(src, AudioSource::Silence);
        assert!(note.is_some());
    }

    #[test]
    fn no_audio_is_silence_without_a_complaint() {
        let (src, note) = resolve_audio(&Audio::None, &sample());
        assert_eq!(src, AudioSource::Silence);
        assert!(note.is_none());
    }
}
