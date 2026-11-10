//! Hardware decoder preference, done by plugin rank so `playbin3` autoplugs the rest.

use gstreamer as gst;
use gst::prelude::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Name prefixes of hardware video decoder factories on Windows (d3d11, d3d12, NVDEC, QSV),
/// macOS (VideoToolbox) and Linux (VA, V4L2, NVDEC).
const HARDWARE_PREFIXES: &[&str] = &["d3d11", "d3d12", "nv", "qsv", "msdk", "vtdec", "va", "v4l2", "vulkan"];

static ORIGINAL_RANKS: OnceLock<Mutex<HashMap<String, gst::Rank>>> = OnceLock::new();

fn is_hardware(factory: &gst::ElementFactory) -> bool {
    let name = factory.name();
    HARDWARE_PREFIXES.iter().any(|p| name.starts_with(p))
}

/// Process-wide: demotes every hardware video decoder to rank NONE (`prefer == false`) or puts
/// back the ranks it had before this crate touched them (`prefer == true`).
pub(crate) fn prefer_hardware(prefer: bool) {
    let factories = gst::ElementFactory::factories_with_type(
        gst::ElementFactoryType::DECODER | gst::ElementFactoryType::MEDIA_VIDEO,
        gst::Rank::NONE,
    );
    let map = ORIGINAL_RANKS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = map.lock() else { return };
    for factory in factories.iter().filter(|f| is_hardware(f)) {
        let name = factory.name().to_string();
        if prefer {
            if let Some(rank) = map.remove(&name) {
                factory.set_rank(rank);
            }
        } else {
            map.entry(name).or_insert_with(|| factory.rank());
            factory.set_rank(gst::Rank::NONE);
        }
    }
}

/// Process-wide: keeps only hardware decoders whose factory name starts with `prefix` (such as
/// `d3d11`); the other hardware decoders drop to rank NONE. Software decoders are untouched.
pub(crate) fn keep_only(prefix: &str) {
    let factories = gst::ElementFactory::factories_with_type(
        gst::ElementFactoryType::DECODER | gst::ElementFactoryType::MEDIA_VIDEO,
        gst::Rank::NONE,
    );
    let map = ORIGINAL_RANKS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = map.lock() else { return };
    for factory in factories.iter().filter(|f| is_hardware(f)) {
        let name = factory.name().to_string();
        map.entry(name.clone()).or_insert_with(|| factory.rank());
        if !name.starts_with(prefix) {
            factory.set_rank(gst::Rank::NONE);
        }
    }
}

/// Factory name of the first video decoder found inside `bin`, if any.
pub(crate) fn decoder_in(bin: &gst::Bin) -> Option<String> {
    for element in bin.iterate_recurse().into_iter().flatten() {
        let Some(factory) = element.factory() else { continue };
        let klass = factory.klass();
        if klass.contains("Decoder") && klass.contains("Video") {
            return Some(factory.name().to_string());
        }
    }
    None
}
