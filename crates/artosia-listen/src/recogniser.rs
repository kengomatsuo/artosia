//! The seam between the listener and a speech recogniser. One implementation exists, whisper.cpp
//! through whisper-rs, behind the `whisper` feature.

use std::path::Path;

/// Turns 16 kHz mono samples into text.
pub trait Recogniser: Send {
    fn transcribe(&mut self, samples: &[f32], is_final: bool) -> Result<String, String>;
}

/// Loads the recogniser for `model`, or says why it cannot.
pub fn load(model: &Path, language: &str) -> Result<Box<dyn Recogniser>, String> {
    #[cfg(feature = "whisper")]
    {
        whisper::Whisper::load(model, language).map(|w| Box::new(w) as Box<dyn Recogniser>)
    }
    #[cfg(not(feature = "whisper"))]
    {
        let _ = (model, language);
        Err("this build has no speech recogniser (built without the `whisper` feature)".to_string())
    }
}

#[cfg(feature = "whisper")]
mod whisper {
    use std::path::Path;

    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

    use super::Recogniser;
    use crate::segment::audio_ctx_for;

    /// With a model larger than the first, a phrase that ended is read with a beam this wide
    /// and no second reading at a higher temperature. On eight real recordings of sung
    /// Indonesian that named six songs where the plain search named five; the first model
    /// named most as it is, with the plain search and the second reading (README, "Measured:
    /// models and settings").
    const BEAM: i32 = 5;
    /// The first model's audio layers, as the file itself reports them.
    const FIRST_LAYERS: i32 = 6;
    /// A reading that takes more than this share of the phrase's own length is too slow, and
    /// after two of them in a row the plain search is used until the listener starts again.
    const TOO_SLOW: f32 = 0.6;

    pub struct Whisper {
        // The state holds a reference-counted handle to the context, so the model lives as long as it.
        state: WhisperState,
        language: Option<String>,
        threads: i32,
        /// The model is larger than the first, so finished phrases get the beam.
        larger: bool,
        /// Slow readings in a row; at two the beam is given up.
        slow: u8,
    }

    impl Whisper {
        pub fn load(model: &Path, language: &str) -> Result<Self, String> {
            let ctx = WhisperContext::new_with_params(model, WhisperContextParameters::default())
                .map_err(|e| format!("could not load the speech model: {e}"))?;
            let larger = ctx.model_n_audio_layer() > FIRST_LAYERS;
            let state = ctx.create_state().map_err(|e| format!("could not start the recogniser: {e}"))?;
            let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
            // Leave one core for the operator window; whisper gains little past four.
            let threads = cores.saturating_sub(1).clamp(1, 4) as i32;
            let language = match language {
                "" | "auto" => None,
                other => Some(other.to_string()),
            };
            Ok(Self { state, language, threads, larger, slow: 0 })
        }
    }

    impl Recogniser for Whisper {
        fn transcribe(&mut self, samples: &[f32], is_final: bool) -> Result<String, String> {
            // A progress read is thrown away when the phrase ends, so it gets the plain search.
            let beam = self.larger && is_final && self.slow < 2;
            let strategy = if beam { SamplingStrategy::BeamSearch { beam_size: BEAM, patience: -1.0 } } else { SamplingStrategy::Greedy { best_of: 1 } };
            let mut params = FullParams::new(strategy);
            if beam {
                params.set_temperature_inc(0.0);
            }
            params.set_language(self.language.as_deref());
            params.set_n_threads(self.threads);
            params.set_translate(false);
            params.set_no_context(true);
            params.set_single_segment(true);
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            // Encode only the seconds that hold audio instead of the full 30 s window.
            params.set_audio_ctx(audio_ctx_for(samples.len()));
            let began = std::time::Instant::now();
            self.state.full(params, samples).map_err(|e| format!("recognition failed: {e}"))?;
            if beam {
                let length = samples.len() as f32 / crate::audio::RECOGNISER_RATE as f32;
                self.slow = if began.elapsed().as_secs_f32() > TOO_SLOW * length.max(1.0) { self.slow + 1 } else { 0 };
            }
            let mut text = String::new();
            for segment in self.state.as_iter() {
                let piece = segment.to_str_lossy().map_err(|e| format!("bad text from the recogniser: {e}"))?;
                text.push_str(&piece);
            }
            Ok(text)
        }
    }
}
