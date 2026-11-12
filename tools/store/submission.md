# What is typed into Partner Center for a submission

Artosia's Store product is `9NTBWLQ5PBHN` (publisher matsuokengo). These texts go into the
submission by hand; the listing texts are `tools/store/listing/<code>.json`.

## Notes for certification (Additional testing information)

Artosia is a free, open-source worship presentation program: it puts song words, Bible verses and announcements on a church's projector. The source is at https://github.com/kengomatsuo/artosia (GPL-3.0-or-later). It has no account, no sign-in and no purchase.

How to try it. The first start asks eight short questions (language, country, a Bible to download, where the church's songs are, which languages the church sings in, the projector). Each is already answered; press the main button on each page, or press "Skip setup" at the top right to go straight to the window. The Bible page has one Bible already picked, with its own copyright line shown: Next downloads it from ebible.org or the Beblia project, and Skip for now downloads nothing. In the window, press Ctrl K, type a few letters of a song or a Bible reference such as John 3:16, press Enter to open it in Preview, then press Show. With one monitor only the operator's window opens; with a second monitor the projector window fills it.

runFullTrust. Artosia is a classic Win32 desktop program written in Rust and packaged as MSIX. It draws its windows with Direct3D, starts its own helper programs from the package for the projector window and the live stream, and loads the GStreamer libraries bundled in the package to decode video.

Firewall rule. The package declares one inbound rule, TCP 4780, for the phone remote: a web page the program serves to phones on the same local network. It is off until the person switches Phone on, and a phone must enter a four-digit code shown on the computer.

Microphone and camera. The microphone is used only while the person switches Listen on; speech is read on the computer itself and nothing is sent. A camera is used only when the person picks one as the background behind the words. Privacy page: https://artosia.matsuokengo.com/privacy/

Updates and data. The Store edition never updates itself. It keeps the church's library in Documents\Artosia so that uninstalling does not delete a church's songs. The same program is also offered as a direct download at https://artosia.matsuokengo.com.

## Why the package needs runFullTrust (Submission options, restricted capabilities)

Artosia is a classic Win32 desktop application packaged with MSIX. It needs full trust to draw its windows with Direct3D through a native UI library, to start its own helper executables from the package (the projector window and the live stream run as separate processes), to load the bundled GStreamer libraries for hardware video decoding, to read songs from other programs' files the user picks, and to serve its phone remote on the local network.

## Fields

| Field | Value |
|---|---|
| Price | Free, all markets |
| Category | Productivity |
| Personal information | Yes, with the privacy page above |
| Website | https://artosia.matsuokengo.com/ |
| Support contact | https://github.com/kengomatsuo/artosia/issues |
| Copyright | Copyright (C) 2026 Kenneth Johannes Fang. GPL-3.0-or-later |
| Additional licence terms | Artosia is free software under the GNU General Public License, version 3 or later. Its source code is at https://github.com/kengomatsuo/artosia. It bundles GStreamer (LGPL) and other libraries listed in the repository. |

## Only the account's owner does these

- Accepting Microsoft's agreements.
- The age-rating questionnaire (it takes the owner's e-mail address).
