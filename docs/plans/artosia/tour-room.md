# The tour runs on the practice library (TOURROOM1, owner-asked 2026-11-14)

The owner expects the tour after setup, and every tour started again, to show the made-up songs,
plan, calendar, members, offerings and announcements, not an empty library. Today the made-up data
loads only in practice mode (`App::load_in(None)`, `--practice`); a church's tour runs over its own,
often empty, library (`ui/tour.rs` `start_tour`, `main.rs` start).

## Decision (this session, delegated by the owner for the partial case)

| Case | Data it runs on | Why |
|---|---|---|
| The tour (after setup, at the first start, Take the tour, Settings, search) | the practice library, swapped in for the tour's length; the church's library comes back unchanged when it ends or is skipped | the tour shows every part with something in it, and writes nothing into the church's own library |
| A lesson from a help mark or What's new | the church's own data | the person is mid-task; a lesson only opens windows and forms and saves nothing itself |

The church's library is never replaced, cleared or seeded: a church that has 1,232 songs must find
them all after a tour.

## Build

```
STATE    App gains `church: Option<Box<App>>`: the church's app parked while the tour shows practice
SWAP     start_tour on an app with a folder: practice = App::load_in(None) (make it pub(crate)),
         carry over what the runtime set on the church's app (image cache, window size, theme,
         language, reduced motion and every shared handle `main.rs` and `view` assign), then
         church = mem::replace(self, practice); self.church = Some(Box::new(church)); tour = Some(0)
END      end_tour: take church back; church.settings.toured = true; touch_settings; *self = church
OUTPUT   the practice app's link is never started, so the projector keeps its last picture and nothing
         the tour shows reaches the room; the church's remote and listener wait unpolled meanwhile
SHOWN    the status bar says the tour shows made-up songs and the library comes back after (new text,
         brief, ten languages)
TESTS    a folder app's tour shows the practice songs; Skip and Done give back the church's songs,
         plan and open song exactly; nothing of the practice library is saved in the church's store
VERIFY   cargo test -p artosia tour; tools/check-lessons.py basics; render the tour after setup on a
         temporary library (`--data <empty folder>`) and with the church's practice library
```
