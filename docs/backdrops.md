# Backdrops: 318 ready pictures and videos behind the words

Researched and built 2026-11-11. The owner asked for at least 250 Christian, nature or otherwise fitting backdrops for lyrics, inside Artosia. A volunteer used to choose a colour, a file of their own or a camera. Now the Background part of the window can offer a catalogue to browse and press. This page is a reading of each source's published terms, not legal advice. Every quote comes from a page fetched that day.

## What Artosia does

| Question | Answer | Where |
|---|---|---|
| What ships in the program | A JSON list of addresses, 256 KB. No picture, no video, no thumbnail. | `crates/artosia-core/assets/backdrops.json` |
| Where a file comes from | Wikimedia's own servers (`thumb.wikimedia.org` for a 1920 wide picture, `upload.wikimedia.org` for a video), on the volunteer's press. | `crates/artosia/src/backdrops.rs` (`download`) |
| Where it lands | `<library>/media/backdrops/<id>.jpg` or `.webm`, written to a `.part` file and renamed when whole. A second press finds it and fetches nothing. | `artosia_core::backdrops::local_path` |
| Previews | 500 px wide, fetched when a row of tiles comes on screen (three at a time), kept in `<library>/media/backdrops/previews`. | `show_rows` in `backdrops.rs` |
| What the button does | "Use" sets the look's background to the local file through `set_background_file` (`Background::Image` or `Background::Video`), downloading first when needed. A bar and a Cancel button show while it runs. Offline gives a toast. | `use_backdrop`, `ui/backdrops.rs` |
| What the browser shows | Category chips, a search field, a grid of equal previews, a mark on tiles already on this computer, and a large preview with author, licence and the page that states it. | `ui/backdrops.rs` (`panel`) |
| Where it opens | From the Design dialog's background row (the Backdrops button beside Choose a file) and from the search (the Backdrops command). It is a dialog of its own, one step inside Design: closing it, or pressing Use, goes back to Design, where the sample shows the new background and the dimming is. | `ui/sheets.rs` (`looks`), `ui/backdrops.rs` (`view`), `back_to_design` |
| A hint for dimming | Mean brightness and detail of the preview, 0 to 100. A small sun mark shows on the brightest and busiest tenth. | `Entry::wants_dimming` |

## Law first

Artosia is GPL-3.0 and anyone may redistribute it. It lists a backdrop only when the file's licence lets Artosia do that: list it in a catalogue, download it on a press from the source's own server, and show author and licence. Each source below was checked on its own licence page.

| Source | Deciding words | URL | Decision |
|---|---|---|---|
| Wikimedia Commons, CC0 files | "You can copy, modify, distribute and perform the work, even for commercial purposes." | https://creativecommons.org/publicdomain/zero/1.0/ | **Used.** This is the CC0 deed that each file's record points to. |
| Wikimedia Commons, what it accepts | "Wikimedia Commons only accepts free content" | https://commons.wikimedia.org/wiki/Commons:Licensing | Used. The same page counts CC0 as public domain: "Material released under a license like CC0 is considered the equivalent of public domain material" |
| Wikimedia API | "Make your requests in series rather than in parallel." and "Set an informative User-Agent string with contact information" | https://www.mediawiki.org/wiki/API:Etiquette | Followed. `tools/backdrops.py` runs one request at a time, two seconds apart, with a User-Agent, and backs off on 429 and 5xx. |
| NASA | "generally are not subject to copyright in the United States." | https://www.nasa.gov/nasa-brand-center/images-and-media/ | **Not used directly.** The word "generally" is not a statement about one file, and the images API gives no licence field per item. NASA pictures that Commons holds carry a per-file public domain tag, so they can enter through Commons. None did in this run. |
| Openverse | It "cannot make any claims about the accuracy of license information" | https://docs.openverse.org/api/reference/made_with_ov.html | **Not used.** It is an index over other sources, and its own page asks users to verify the licence elsewhere. Commons is the primary record. |
| Pexels, the licence | "Don't redistribute or sell the photos and videos on other stock photo or wallpaper platforms" | https://www.pexels.com/license/ | **Not in the catalogue.** A catalogue of Pexels files that Artosia offers is the platform this sentence rules out. |
| Pexels, the API | "You may not copy or replicate core functionality of Pexels" | https://www.pexels.com/api/documentation/ | Needs a key. See "Not built". |
| Pixabay, the licence | "You cannot sell or distribute Content" (on its own, unaltered) | https://pixabay.com/service/license-summary/ | **Not in the catalogue.** |
| Pixabay, the API | "permanent hotlinking of images (using Pixabay URLs in your app) is not allowed." and "Systematic mass downloads are not allowed." | https://pixabay.com/api/docs/ | Needs a key, and a built catalogue would be a systematic download. See "Not built". |
| Unsplash, the licence | "Compiling images from Unsplash to replicate a similar or competing service." (listed as not allowed) | https://unsplash.com/license | **Not in the catalogue.** |
| Unsplash, the API | "require the image URLs returned by the API to be directly used" | https://unsplash.com/documentation | Needs a key and a download-tracking call. See "Not built". |

What Artosia may do with a Commons CC0 file: list its address, fetch it on a press, store it in the library, show it on the projector, and show its author and licence next to it. CC0 asks for no credit, and the browser shows one anyway.

What it may not do, and does not: change the licence line, hide the author, offer a file whose record carries a usage restriction, or ship a file inside the program. The program never contains a picture.

Some Commons files carry "(Unsplash)" in their name (65 of the 318). Photos that Unsplash published before June 2017 were under CC0, and Commons holds them as CC0. Their Commons record says CC0, which is what the script checks.

## How the catalogue is built

`python tools/backdrops.py` rebuilds `backdrops.json`. It needs Python 3 and, for the two hints, Pillow. Nothing else is installed, and no key is involved.

1. For each category it runs searches against `https://commons.wikimedia.org/w/api.php` limited to CC0 files (`incategory:CC-zero`), pictures of 1920 pixels or more (`filew:>1919`) or videos.
2. For every hit it reads the API's own answer for that file: `LicenseShortName` must read CC0 or public domain, `Artist` must be present, and `Restrictions` must be empty. The licence address comes from the same record. A search snippet decides nothing.
3. A picture must be JPEG, at least 1920 by 1080 and at least 1.3 times wider than tall, and its 1920 wide copy is the address kept. A video must be 4 to 90 seconds and 848 pixels wide or more. The file kept is the smallest VP9 WebM transcode of 720 lines or more, else the largest smaller one. Anything over 45 MB is left out.
4. Each preview is fetched once to compute brightness and detail.
5. `tools/backdrops-exclude.txt` lists 283 files a person looked at in contact sheets of every preview and turned down, with the reason. A turned-down file still takes its place in a search's quota, so a rebuild does not pull in next-ranked files nobody has looked at. When Commons changes its ranking, new files appear and need the same look before they are committed.

What the people who looked turned down: drawings, paintings and museum objects; photographs with a recognisable person or animal (the people category lost 20 of 25); buildings, streets, vehicles, pylons and airports; pictures with a date, a watermark or a caption; near copies of a neighbour; empty or black skies; a lantern festival; videos of sport, machines, crowds, charts and close-ups.

### Counts, 318 in all

| Category | Pictures | Videos | Total |
|---|---|---|---|
| Sky and clouds | 46 | 11 | 57 |
| Mountains | 41 | 3 | 44 |
| Sea and water | 44 | 9 | 53 |
| Forest and fields | 40 | 1 | 41 |
| Light and sun rays | 19 | 1 | 20 |
| Stars and night | 31 | 4 | 35 |
| Candles and warm light | 6 | 3 | 9 |
| Cross and church | 44 | 1 | 45 |
| Hands and people at worship | 5 | 0 | 5 |
| Plain textures and soft gradients | 9 | 0 | 9 |
| Total | 285 | 33 | 318 |

By source: Wikimedia Commons 318. By licence: CC0 316, public domain 2. By author: 225 different people. Pictures are 1920 pixels wide (88 are 3:2, 58 are 4:3, 20 are 16:9, the rest close to those). Videos weigh 237 MB together and 20 MB at most; 24 are 1080 lines and 9 are 480 lines.

Thin spots, with the reason: hands and people at worship has 5 because nearly every picture of people shows a recognisable face; textures has 9 and candles 9 because Commons holds few CC0 files for them; church has one video because the church videos on Commons are mostly of people.

## What can play

`tools/gstreamer-subset.py` bundles `matroska` (WebM), `vpx` (libvpx VP8 and VP9), `d3d11` (`d3d11vp9dec` hardware decode of VP9 on the graphics chip), `libav`, and `dav1d`. `crates/artosia-video` plays with `flags=video`, so a WebM's Opus sound track is never decoded. The Windows machine this was built on lists `d3d11vp9dec` and `vp9dec`. `set_background_file` already accepts `webm`.

H.264 in MP4, which the release plays for certain, is not offered: Commons does not host MP4, and its own H.264-free transcodes are WebM. So every video here is VP9 in WebM, and that is the thing to watch on a budget PC (see "Not verified").

## Not built

- **An optional key for Pexels, Pixabay or Unsplash.** None of the three may be listed in a catalogue (above). Their APIs would each need a key typed into settings, a licence-compliant way to show credit, and for Unsplash a tracking call per download, and Pixabay forbids permanent hotlinks. That is a second feature with its own screen. It was left out and is the owner's call. The catalogue's entry shape (`source`, `licence`, `licence_url`, `page`) already carries what a second source would need.
- **People at worship.** Five pictures. The licence permits more; the pictures do not exist as CC0 without faces.

## Not verified

- A video backdrop was played in `artosia-video`'s loop example (table below) and not yet watched on the projector window of Artosia itself.
- The grid was rendered with made-up entries and generated previews. A scroll through real previews on a real connection was not watched.
- The large preview of a video shows its poster frame, not the motion.
- Commons may delete or rename a file. The press then fails with a message, and files already downloaded keep working.
- Whether a Commons CC0 mark is right for every file is the uploader's claim and Commons' check, not Artosia's.

## What a video backdrop cost here (2026-11-11)

Two catalogue files, fetched from Commons, played by `target/release/examples/loop.exe` until the position had wrapped twice, then measured for 20 seconds. Intel Iris Xe laptop; the window was minimised, so these are decode and upload costs and are lower than the drawn figures in [output.md](output.md).

| File | Size on screen | Decoder | Cores | Working set |
|---|---|---|---|---|
| A candle, 21 s | 1920x1080 VP9 | `d3d11vp9dec` (graphics chip) | 0.17 | 242 MB |
| The same | 1920x1080 VP9 | `vp9dec` (software) | 0.34 | 219 MB |
| Street lamps, 10 s | 848x480 VP9 | `d3d11vp9dec` | 0.23 | 214 MB |
| The same | 848x480 VP9 | `vp9dec` (software) | 0.24 | 181 MB |
| `fixtures/bg30.mp4`, for comparison | 1920x1080 H.264 | `avdec_h264` (software) | 0.45 | 341 MB |

So VP9 in software cost less here than H.264 in software, and a PC whose graphics chip cannot decode VP9 still plays the 1080 line files. This PC is not the budget target; the number to trust comes from the target machine.

## Sources

All read on 2026-11-11: https://commons.wikimedia.org/wiki/Commons:Licensing, https://creativecommons.org/publicdomain/zero/1.0/, https://www.mediawiki.org/wiki/API:Etiquette, https://www.nasa.gov/nasa-brand-center/images-and-media/, https://docs.openverse.org/api/reference/made_with_ov.html, https://www.pexels.com/license/, https://www.pexels.com/api/documentation/, https://pixabay.com/service/license-summary/, https://pixabay.com/api/docs/, https://unsplash.com/license, https://unsplash.com/documentation, and the Commons API (`action=query`, `generator=search`, `prop=imageinfo|videoinfo`).
