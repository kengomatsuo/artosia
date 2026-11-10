//! Moves the output windows when the screens change, without a gap in the picture.
//!
//! A change opens the new window first and closes the old one only after the new one has drawn
//! its first frame, so the room never waits on a restart. Rune cannot change a live window
//! between bordered and borderless, and its window handle has no such call, so "moving" a window
//! is opening the right kind of window in the same process (`rune::windows::open_window`) and
//! closing the old one. Everything here runs on the event thread, called from each window's poll.
//!
//! While a window opens the video is held back. Rune configures a new window's surface by
//! waiting for an idle GPU, and wgpu panics if a video frame completes meanwhile
//! (`GpuWaitTimeout`, seen with a second window opened over playing video).

use std::cell::RefCell;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use artosia_slides::frame::trace;
use rune::WindowHandle;

use crate::displays::{self, Plan, Spot};
use crate::{Role, make_spec, shared};

/// How long the video stays off before a window opens, so frames in flight finish.
const QUIET: Duration = Duration::from_millis(150);
/// The video never stays off longer than this, whatever happens to the new window.
const QUIET_LIMIT: Duration = Duration::from_secs(6);

struct Live {
    /// None until Rune has created a window that the start list made.
    handle: Option<WindowHandle>,
    spot: Spot,
    opened: Instant,
    /// A full-screen window starts hidden and is shown once it covers its screen exactly.
    settled: bool,
    fixes: u8,
}

impl Live {
    fn new(handle: Option<WindowHandle>, spot: Spot) -> Live {
        Live { handle, spot, opened: Instant::now(), settled: false, fixes: 0 }
    }

    /// Windows moves a new window to its screen at that screen's scale, which can leave it
    /// bigger or smaller than the screen. Put it exactly on the screen, then show it.
    fn settle(&mut self, role: Role) {
        if self.settled {
            return;
        }
        let Some(handle) = &self.handle else { return };
        let Some(at) = handle.placement() else { return };
        // Off the taskbar while the window is still hidden, then shown.
        let Spot::Screen { disp, .. } = &self.spot else {
            displays::hide_from_taskbar(handle);
            handle.show();
            self.settled = true;
            trace(|| format!("{}: window shown after {} ms", name(role), self.opened.elapsed().as_millis()));
            return;
        };
        let near = |a: i32, b: i32| (a - b).abs() <= 1;
        let (want_at, want_size) = (disp.position, disp.size);
        let size = at.content_size;
        let right = near(at.position.0, want_at.0) && near(at.position.1, want_at.1) && near(size.0 as i32, want_size.0 as i32) && near(size.1 as i32, want_size.1 as i32);
        let gave_up = self.fixes >= 8 || self.opened.elapsed().as_secs() >= 4;
        if !right && !gave_up {
            self.fixes += 1;
            let scale = at.scale.max(0.1) as f32;
            trace(|| {
                format!(
                    "{}: window is {}x{} at {},{} (scale {scale}), screen is {}x{} at {},{}: fixing",
                    name(role),
                    size.0,
                    size.1,
                    at.position.0,
                    at.position.1,
                    want_size.0,
                    want_size.1,
                    want_at.0,
                    want_at.1
                )
            });
            handle.set_position(want_at);
            handle.resize_instant((want_size.0 as f32 / scale, want_size.1 as f32 / scale));
            return;
        }
        if gave_up && !right {
            trace(|| format!("{}: could not fit the window to the screen, showing it as it is", name(role)));
        }
        displays::hide_from_taskbar(handle);
        handle.show();
        self.settled = true;
        trace(|| format!("{}: covers {} exactly after {} fixes, shown at {} ms", name(role), spot_name(&self.spot), self.fixes, self.opened.elapsed().as_millis()));
    }
}

struct Slot {
    role: Role,
    current: Option<Live>,
    /// A window that is opening and has not drawn yet.
    pending: Option<Live>,
    /// A window to open once the video has been quiet for a moment.
    waiting: Option<(Spot, u32, Instant)>,
}

struct Placer {
    epoch: u64,
    serial: u32,
    slots: [Slot; 2],
    /// Since when the video is held back for a window change; None when it is not.
    quiet_since: Option<Instant>,
}

thread_local! {
    static PLACER: RefCell<Option<Placer>> = const { RefCell::new(None) };
}

fn name(role: Role) -> &'static str {
    match role {
        Role::Audience => "projector",
        Role::Stage => "stage",
        Role::Anchor => "anchor",
    }
}

fn spot_name(spot: &Spot) -> String {
    match spot {
        Spot::Window => "a window".to_string(),
        Spot::Screen { number, disp } => format!("screen {number} ({})", disp.name),
    }
}

fn video_live() -> bool {
    shared().video_live.load(Ordering::Acquire)
}

/// Records the windows the start list opens. Call on the event thread before they exist.
pub fn init(plan: &Plan) {
    let live = |spot: &Spot| Live::new(None, spot.clone());
    let slots = [
        Slot { role: Role::Audience, current: plan.audience.as_ref().map(live), pending: None, waiting: None },
        Slot { role: Role::Stage, current: plan.stage.as_ref().map(live), pending: None, waiting: None },
    ];
    PLACER.with(|placer| *placer.borrow_mut() = Some(Placer { epoch: displays::epoch(), serial: 0, slots, quiet_since: None }));
}

/// Gives the placer the handle of a window the start list opened.
pub fn adopt(role: Role, handle: WindowHandle) {
    PLACER.with(|placer| {
        if let Some(placer) = placer.borrow_mut().as_mut()
            && let Some(live) = placer.slots.iter_mut().find(|slot| slot.role == role).and_then(|slot| slot.current.as_mut())
        {
            live.handle = Some(handle);
        }
    });
}

/// Whether a window is still opening or being fitted, so the poll should come back soon.
pub fn busy() -> bool {
    PLACER.with(|placer| placer.borrow().as_ref().is_some_and(|placer| placer.slots.iter().any(Slot::busy)))
}

/// Every two seconds, traces how many frames each window draws and presents a second, and how
/// many draws the video surface has seen.
pub fn trace_stats() {
    thread_local! {
        static LAST: RefCell<Option<Instant>> = const { RefCell::new(None) };
    }
    let due = LAST.with(|last| {
        let mut last = last.borrow_mut();
        let due = last.is_none_or(|at| at.elapsed().as_secs() >= 2);
        if due {
            *last = Some(Instant::now());
        }
        due
    });
    if !due {
        return;
    }
    PLACER.with(|placer| {
        let Ok(placer) = placer.try_borrow() else { return };
        let Some(placer) = placer.as_ref() else { return };
        for slot in &placer.slots {
            if let Some(handle) = slot.current.as_ref().and_then(|live| live.handle.as_ref()) {
                trace(|| format!("frames {}: {}", name(slot.role), handle.surface_stats().summary()));
                if slot.role == Role::Audience
                    && let Some(at) = handle.placement()
                {
                    let (w, h, scale) = *shared().last_view.lock().map(|last| *last).as_ref().unwrap_or(&(0.0, 0.0, 0.0));
                    let (real_w, real_h) = (at.content_size.0 as f64 / at.scale, at.content_size.1 as f64 / at.scale);
                    let same = (f64::from(w) - real_w).abs() < 1.5 && (f64::from(h) - real_h).abs() < 1.5;
                    // Said when it changes, not every two seconds.
                    let line = format!("size check: window is {real_w:.0}x{real_h:.0} points at scale {:.2}, the view is laid out for {w}x{h} at {scale}: {}", at.scale, if same { "same" } else { "DIFFERENT" });
                    thread_local! {
                        static SAID: RefCell<String> = const { RefCell::new(String::new()) };
                    }
                    SAID.with(|said| {
                        if *said.borrow() != line {
                            trace(|| line.clone());
                            *said.borrow_mut() = line;
                        }
                    });
                }
            }
        }
    });
    trace(|| {
        let (submitted, completed) = shared().surface.counts();
        format!("video surface draws: submitted {submitted}, completed {completed}")
    });
}

/// Reacts to a changed screen list and finishes windows that have drawn. Cheap when nothing
/// changed.
pub fn tick() {
    PLACER.with(|placer| {
        let Ok(mut placer) = placer.try_borrow_mut() else { return };
        let Some(placer) = placer.as_mut() else { return };
        let epoch = displays::epoch();
        if epoch != placer.epoch {
            placer.epoch = epoch;
            placer.react();
        }
        placer.hidden_by_person();
        placer.hold_video();
        let video_quiet = placer.quiet_since.is_some() || !video_live();
        for slot in &mut placer.slots {
            slot.open_due(video_quiet);
            slot.finish();
        }
        placer.hold_video();
    });
}

impl Placer {
    /// Holds the video back from the moment a window is wanted until all are done.
    fn hold_video(&mut self) {
        let busy = self.slots.iter().any(Slot::busy);
        let shared = shared();
        match (busy, self.quiet_since) {
            (true, None) if video_live() => {
                self.quiet_since = Some(Instant::now());
                shared.quiet.store(true, Ordering::Release);
                shared.touch();
                trace(|| "video held back while a window opens".to_string());
            }
            (false, Some(since)) => {
                self.quiet_since = None;
                shared.quiet.store(false, Ordering::Release);
                shared.touch();
                trace(|| format!("video back after {} ms", since.elapsed().as_millis()));
            }
            (true, Some(since)) if since.elapsed() > QUIET_LIMIT => {
                // A window that never finishes must not keep the video off for good.
                self.quiet_since = None;
                shared.quiet.store(false, Ordering::Release);
                shared.touch();
                trace(|| "video back at the limit, a window is still not done".to_string());
            }
            _ => {}
        }
    }

    fn react(&mut self) {
        let list = displays::current();
        let plan = displays::plan(&list, &displays::pins());
        let none = |spot: &Option<Spot>| spot.as_ref().map_or("none".to_string(), spot_name);
        trace(|| format!("decision: projector -> {}, stage -> {} ({} screens)", none(&plan.audience), none(&plan.stage), list.len()));
        crate::set_layout(list, plan.audience_index, plan.stage_index);
        for (slot, want) in self.slots.iter_mut().zip([plan.audience, plan.stage]) {
            self.serial += 1;
            slot.want(want, self.serial);
        }
    }

    /// The person at the projector window pressed Escape or clicked it: hide it for good, until
    /// the operator asks for it again.
    fn hidden_by_person(&mut self) {
        if !shared().hide_request.swap(false, Ordering::AcqRel) {
            return;
        }
        let mut pins = displays::pins();
        if !pins.here {
            return;
        }
        pins.here = false;
        trace(|| "the projector window was hidden by the person at it".to_string());
        displays::set_pins(pins, || shared().touch());
        shared().tell(&artosia_slides::frame::FromOutput::ProjectorHidden);
    }
}

impl Slot {
    fn busy(&self) -> bool {
        self.pending.is_some() || self.waiting.is_some() || self.current.as_ref().is_some_and(|live| !live.settled)
    }

    fn want(&mut self, spot: Option<Spot>, serial: u32) {
        let heading_for = self.waiting.as_ref().map(|(spot, ..)| spot).or_else(|| self.pending.as_ref().or(self.current.as_ref()).map(|live| &live.spot));
        if heading_for == spot.as_ref() {
            return;
        }
        self.waiting = None;
        // A window still opening for a place nobody wants any more is dropped.
        if let Some(stale) = self.pending.take() {
            if let Some(handle) = stale.handle {
                handle.close();
            }
            trace(|| format!("{}: dropped the window that was opening on {}", name(self.role), spot_name(&stale.spot)));
        }
        let Some(spot) = spot else {
            // No window wanted: close what is there.
            if let Some(old) = self.current.take()
                && let Some(handle) = old.handle
            {
                handle.close();
                trace(|| format!("{}: closed the window on {}", name(self.role), spot_name(&old.spot)));
            }
            return;
        };
        if self.current.as_ref().is_some_and(|live| live.spot == spot) {
            return;
        }
        // With video playing the new window waits a moment, so no frame is in flight when its
        // surface is configured.
        let delay = if video_live() { QUIET } else { Duration::ZERO };
        self.waiting = Some((spot, serial, Instant::now() + delay));
    }

    /// Opens the window that waited for the video to go quiet, once the wait is over.
    fn open_due(&mut self, video_quiet: bool) {
        let Some((_, _, due)) = &self.waiting else { return };
        if Instant::now() < *due || !video_quiet {
            return;
        }
        let Some((spot, serial, _)) = self.waiting.take() else { return };
        let key = format!("{}-{serial}", name(self.role));
        let since_change = displays::changed_at().map(|at| at.elapsed().as_millis()).unwrap_or(0);
        trace(|| format!("{}: opening {} for {} ({since_change} ms after the change)", name(self.role), key, spot_name(&spot)));
        let handle = rune::windows::open_window(make_spec(self.role, key, &spot, true));
        self.pending = Some(Live::new(Some(handle), spot));
    }

    fn finish(&mut self) {
        let role = self.role;
        if let Some(current) = self.current.as_mut() {
            current.settle(role);
        }
        if let Some(pending) = self.pending.as_mut() {
            pending.settle(role);
        }
        let Some(pending) = &self.pending else { return };
        if !pending.settled || !pending.handle.as_ref().is_some_and(WindowHandle::is_ready) {
            if pending.opened.elapsed().as_secs() >= 10 {
                trace(|| format!("{}: the new window has not drawn after 10 s", name(self.role)));
            }
            return;
        }
        // The old window goes only once its handle exists; until then both stay.
        if self.current.as_ref().is_some_and(|live| live.handle.is_none()) {
            return;
        }
        let ready_ms = pending.opened.elapsed().as_millis();
        if let Some(old) = self.current.take()
            && let Some(handle) = old.handle
        {
            handle.close();
        }
        self.current = self.pending.take();
        let total = displays::changed_at().map(|at| at.elapsed().as_millis()).unwrap_or(0);
        let spot = self.current.as_ref().map(|live| spot_name(&live.spot)).unwrap_or_default();
        trace(|| format!("{}: now on {spot}; the new window drew after {ready_ms} ms, old one closed {total} ms after the change", name(self.role)));
    }
}
