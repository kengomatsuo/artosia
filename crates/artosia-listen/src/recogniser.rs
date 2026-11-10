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

    pub struct Whisper {
        // The state holds a reference-counted handle to the context, so the model lives as long as it.
        state: WhisperState,
        language: Option<String>,
        threads: i32,
    }

    impl Whisper {
        pub fn load(model: &Path, language: &str) -> Result<Self, String> {
            let ctx = WhisperContext::new_with_params(model, WhisperContextParameters::default())
                .map_err(|e| format!("could not load the speech model: {e}"))?;
            let state = ctx.create_state().map_err(|e| format!("could not start the recogniser: {e}"))?;
            let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
            // Leave one core for the operator window; whisper gains little past four.
            let threads = cores.saturating_sub(1).clamp(1, 4) as i32;
            let language = match language {
                "" | "auto" => None,
                other => Some(other.to_string()),
            };
            Ok(Self { state, language, threads })
        }
    }

    impl Recogniser for Whisper {
        fn transcribe(&mut self, samples: &[f32], _is_final: bool) -> Result<String, String> {
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
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
            self.state.full(params, samples).map_err(|e| format!("recognition failed: {e}"))?;
            let mut text = String::new();
            for segment in self.state.as_iter() {
                let piece = segment.to_str_lossy().map_err(|e| format!("bad text from the recogniser: {e}"))?;
                text.push_str(&piece);
            }
            Ok(text)
        }
    }
}
