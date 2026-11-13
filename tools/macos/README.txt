Artosia for Mac (Apple silicon)

1. Drag Artosia onto the Applications folder beside it.
2. Open Artosia from Applications.

The first time, macOS refuses to open it. Do this once:

1. Try to open Artosia. A box says "Artosia.app" Not Opened. Press Done,
   not Move to Bin.
2. Open System Settings, then Privacy & Security.
3. Scroll down and press Open Anyway beside Artosia, then Open.

Why: Artosia is free and pays for no Apple developer programme, so macOS
cannot check who made it. After that one time it opens like any other program.

Artosia updates itself: it fetches the newest release, checks its signature and
swaps it in at the next start. Keep it in Applications, or in a folder of your
own. Opened straight from this disk image it runs, and cannot update.

Your songs, Bibles and settings are kept in your Library folder, under
Application Support, and stay when you delete the app.

To remove Artosia, drag it from Applications to the Bin.

Streaming and recording take the projector's screen. The first time, macOS asks
whether Artosia may record the screen: allow it in System Settings, Privacy &
Security, Screen & System Audio Recording, then open Artosia again.

Video backgrounds, cameras and streaming run on the part of GStreamer inside the app
(gstreamer.freedesktop.org). Its licences are in the app, under
Contents/Resources/gstreamer/licenses.zip.

Source and licence (GPL-3.0-or-later): https://github.com/kengomatsuo/artosia
Homepage: https://artosia.matsuokengo.com
