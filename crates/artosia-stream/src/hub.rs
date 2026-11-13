//! Hands the encoded video and audio to every destination.
//!
//! The encoder pipeline lives for the whole run. Each destination has its own small pipeline
//! (mux and sink), so one lost connection never touches the others or the encoder. The hub is
//! the join: the encoder's output goes in, every destination reads its own queue.

use gstreamer as gst;
use gstreamer::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};

/// How many packets one destination may fall behind before it skips ahead to a keyframe.
/// Thirty seconds of video and audio at 30 fps.
const QUEUE_PACKETS: usize = 2500;

/// One piece of encoded output.
pub enum Packet {
    Video(gst::Sample),
    Audio(gst::Sample),
    /// The encoder has finished; no more follow.
    End,
}

struct Subscriber {
    tx: SyncSender<Packet>,
    overflowed: Arc<AtomicBool>,
}

/// What a destination holds to read the stream.
pub struct Subscription {
    pub rx: Receiver<Packet>,
    /// Set by the hub when this reader fell too far behind and lost packets.
    pub overflowed: Arc<AtomicBool>,
}

/// Counters and the list of readers.
#[derive(Default)]
pub struct Hub {
    subscribers: Mutex<Vec<Subscriber>>,
    video_caps: Mutex<Option<gst::Caps>>,
    audio_caps: Mutex<Option<gst::Caps>>,
    encoder: Mutex<Option<gst::Element>>,
    /// Video frames the encoder produced.
    pub frames_encoded: AtomicU64,
    /// Frames lost before or after the encoder.
    pub frames_dropped: AtomicU64,
    /// Ticks that sent the previous picture again because no new one had arrived.
    pub frames_repeated: AtomicU64,
    /// Encoded bytes, video and audio together.
    pub bytes_out: AtomicU64,
    /// Set once the encoder has sent its last packet.
    pub finished: AtomicBool,
    eos_seen: AtomicU64,
}

impl Hub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Remembers the encoder element, so a new destination can ask it for a keyframe.
    pub fn set_encoder(&self, encoder: gst::Element) {
        if let Ok(mut e) = self.encoder.lock() {
            *e = Some(encoder);
        }
    }

    /// Asks the encoder for a keyframe with fresh headers. Safe to call at any time.
    pub fn request_keyframe(&self) {
        let encoder = self.encoder.lock().ok().and_then(|e| e.clone());
        if let Some(encoder) = encoder {
            let event = gstreamer_video::UpstreamForceKeyUnitEvent::builder()
                .all_headers(true)
                .build();
            let _ = encoder.send_event(event);
        }
    }

    /// The caps of the encoded video and audio, once both have produced a packet.
    pub fn caps(&self) -> Option<(gst::Caps, gst::Caps)> {
        let v = self.video_caps.lock().ok()?.clone()?;
        let a = self.audio_caps.lock().ok()?.clone()?;
        Some((v, a))
    }

    pub fn subscribe(&self) -> Subscription {
        let (tx, rx) = sync_channel(QUEUE_PACKETS);
        let overflowed = Arc::new(AtomicBool::new(false));
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push(Subscriber {
                tx,
                overflowed: Arc::clone(&overflowed),
            });
        }
        Subscription { rx, overflowed }
    }

    /// Called when one of the two encoder sinks reaches the end. The second call ends the stream.
    pub fn sink_eos(&self) {
        if self.eos_seen.fetch_add(1, Ordering::SeqCst) + 1 >= 2 {
            self.publish(Packet::End);
        }
    }

    /// Called from the encoder's appsink callbacks.
    pub fn publish(&self, packet: Packet) {
        match &packet {
            Packet::Video(sample) => {
                self.frames_encoded.fetch_add(1, Ordering::Relaxed);
                self.add_bytes(sample);
                if let (Some(caps), Ok(mut slot)) = (sample.caps_owned(), self.video_caps.lock())
                    && slot.is_none()
                {
                    *slot = Some(caps);
                }
            }
            Packet::Audio(sample) => {
                self.add_bytes(sample);
                if let (Some(caps), Ok(mut slot)) = (sample.caps_owned(), self.audio_caps.lock())
                    && slot.is_none()
                {
                    *slot = Some(caps);
                }
            }
            Packet::End => self.finished.store(true, Ordering::SeqCst),
        }
        let Ok(mut subs) = self.subscribers.lock() else {
            return;
        };
        subs.retain(|s| {
            let copy = match &packet {
                Packet::Video(x) => Packet::Video(x.clone()),
                Packet::Audio(x) => Packet::Audio(x.clone()),
                Packet::End => Packet::End,
            };
            match s.tx.try_send(copy) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    s.overflowed.store(true, Ordering::SeqCst);
                    true
                }
                Err(TrySendError::Disconnected(_)) => false,
            }
        });
    }

    fn add_bytes(&self, sample: &gst::Sample) {
        if let Some(buffer) = sample.buffer() {
            self.bytes_out
                .fetch_add(buffer.size() as u64, Ordering::Relaxed);
        }
    }
}
