# The homepage

`artosia.matsuokengo.com` is one static page in each of ten languages (English, Indonesian, Simplified and Traditional Chinese, Korean, Japanese, Spanish, Brazilian Portuguese, French and Vietnamese), and a privacy page in English. Its job is
the one the owner named on 2026-11-11: a volunteer finds the single file to download and
knows what to do with it.

## Where it lives

| Thing | Place |
|---|---|
| The page, once, with `{{key}}` holes | `site/page.html` |
| Its words | `site/strings/<code>.json`, one per language |
| What each string must do | `site/strings/brief.json` |
| The privacy page | `site/privacy.html` and `site/privacy.json`; its facts and their proof are in [privacy-facts.md](privacy-facts.md) |
| Look and motion | `site/style.css`, `site/script.js` |
| Pictures | `site/img/`, made by `tools/site-shots.sh` |
| The built site | `target/site/`, made by `tools/build-site.py` |
| How it is served | `site/deploy/wrangler.jsonc`, deployed by `tools/publish-site.sh` |

## How it is served

A Cloudflare Worker named `artosia-site` serves the built folder and runs no code
(`site/deploy/wrangler.jsonc`: `assets` only, after the house's other sites). Its custom
domain made the DNS record and the certificate by itself, which is why no record was added
by hand: wrangler's own sign-in may not write DNS records, and Cloudflare's page on custom
domains says it "will create DNS records and issue certificates". Requests for static files
are free and unlimited (both pages read 2026-11-12).

wrangler is signed in on the Mac, so `tools/publish-site.sh` sends the built folder there over
ssh from Windows itself (Tailscale) and runs `wrangler deploy` there. Until 2026-11-14 that ssh
went through WSL, whose network could not reach the Mac, and every release since 0.1.28 left the
page behind; 0.1.31 was deployed from WSL's own signed-in wrangler once, before the owner put
WSL off limits. When the Mac cannot be reached the script says so and the site keeps its last
deploy; the page's own script still shows the newest version and sizes, read from GitHub.

GitHub Pages served the first copy for a day (2026-11-11) from a `gh-pages` branch, waiting
for a DNS record at Cloudflare. It was switched off and the branch deleted on 2026-11-12,
so one copy exists.

`tools/publish.sh` ends by running `tools/publish-site.sh`, so each release rewrites the
version and size on the page. The script reads both from the newest release, never from
`Cargo.toml`, so the page cannot name a version nobody can download. The page's own script
then checks the newest release when it is opened and keeps the answer for an hour.

## The download area

The area is named "Get Artosia" and shows your system first (direction A of the 2026-11-12
mock). One row per system that has a file: Windows, Linux and, once it exists, the Mac. A system
shows only when its download exists: no row, no link and no word for it otherwise.

- The visitor's own row comes first and holds the filled button; the hero's button, its file line
  and its "Also for ..." line follow it, and so does the top bar's Download button (class `dl-own`),
  which would otherwise hand a Windows file to a Linux visitor. `script.js` reads `navigator.userAgentData.platform`, then
  `navigator.platform`, and Android counts as neither Linux nor Mac. A Mac visitor to a page with no
  Mac row sees Windows first. Without JavaScript Windows is the filled one and every row stays reachable.
- `?os=linux` and `?os=mac` override the guess. It is how the page is checked: open
  `target/site-en/index.html` and `target/site-en-mac/index.html` from a local server with each value.
- The Windows button points at `https://github.com/kengomatsuo/artosia/releases/latest/download/Artosia-setup.exe`,
  which always answers with the newest release's setup ([setup.md](setup.md)). The zip of the same
  release is the second way, in the Windows row, for a computer with no internet. The Linux row
  leads with the one-line install and a copy button ([linux.md](linux.md)); its link is
  `Artosia-linux-x86_64.tar.gz` (every release from 0.1.22, [linux.md](linux.md)); the Mac link is
  `Artosia-macos-aarch64.dmg` (Apple silicon, every release from 0.1.23, [macos.md](macos.md)).
  Both name the release the page was built for (`releases/download/v<version>/`), never `latest`:
  a release whose Linux build failed carries Windows only (`tools/publish.sh`), and `latest` would
  then answer 404. `tools/publish-site.sh` refuses to build from a release with no Linux package,
  so the page keeps its last deploy and its links keep working.
- Each download button and link carries its system's mark before the words (`{{icon:windows}}`, `{{icon:apple}}`, `{{icon:linux}}` in
  `page.html`; the build puts the SVG of `site/img/icons/` in, drawn in the words' colour), as Audacity's download page does; a mark
  alone, as FreeShow and VLC have it, was left because it cannot be read aloud. The three are Font Awesome Free 6.7.2 (CC BY 4.0;
  each file keeps the licence comment its licence asks for, `site/img/icons/credits.json`), and each mark is its owner's trademark,
  used to name the system a file is for.
- The Mac row says how to install and nothing more since 2026-11-13: the app is signed with a
  Developer ID and notarized, so macOS opens it without a step of its own. Until then the row
  carried three numbered Open Anyway steps.
- A sentence with links is one string per variant (`also_linux`, `also_linux_mac`, and so on),
  with the links written as `{linux}...{/linux}` around the writer's own word, never fragments joined.
- The search result's grey line (`meta_description`, and `meta_description_mac` once the Mac file
  exists) names the systems; the title names Windows alone, the edition a church runs. The line may
  run to 170 characters: Google's page on snippets says a description has no length limit and is cut
  to fit the device (developers.google.com/search/docs/appearance/snippet, read 2026-11-13), and at
  155 three languages clipped their grammar to fit.
- `python -I tools/check-site-strings.py` holds every language against the brief: the same keys,
  the tokens and link markers kept, the visible text within its `max` (a link's markers are not
  counted), and none of the marks the house bans.
- The page's own script also refreshes the Linux size, and the Mac size when the release has it, with the
  version and the Windows sizes, kept for an hour.

## The build's arguments

`python -I tools/build-site.py --version V --setup-bytes N --zip-bytes N --linux-bytes N
[--mac-bytes N] [--langs en] [--out DIR]`. Without `--mac-bytes` (and without
`dist/<version>/Artosia-macos-aarch64.dmg`) the Mac row and its link are left out and the hero names Linux
alone; with it they are built and the hero names both. Both variants are checked on every build: a key of
either variant that a language lacks stops the build. `tools/publish-site.sh` reads the three sizes (and the
Mac's when the release holds it) from the newest release and passes them.

## Languages

`tools/build-site.py` builds the page in every language of its `LANGUAGES`. The top bar holds a language menu: the page's language in its own name, and a list of all ten. Each page carries `hreflang` links to the others. The first page sends a browser to its own language once, by the rule of `crates/artosia-locale` repeated in `REDIRECT` (the same code first, Chinese by script, then the same first part; [i18n/README.md](i18n/README.md)). The privacy page is at `/privacy/` and exists in English only.

## The look

It wears the matsuokengo.com look, and its rules come from that site's own files, saved
with the research in `~/Shared/inspo/artosia-homepage/brand/`: the colours, the 960 px
column, the 24 px corners, Plus Jakarta Sans, the slim nav that grows a line after
scrolling, the buttons whose border turns toward the pointer, and elements that rise as
they come into view. `site/style.css` keeps that file's rules above the line "Artosia's
own" and adds below it.

Three things differ from that site, each on purpose.

| Difference | Why |
|---|---|
| The download button is filled with the ink colour | The owner could not find the download; the brand allows no accent fill, and ink is not the accent |
| Section and feature headings are larger than the portfolio's | They follow the owner's own product page, `undirect.matsuokengo.com` |
| No uppercase tags or chips | The house rule against letter-spaced uppercase labels outranks the brand |

## The pictures are the program

Every picture is a headless render of a real scene (`artosia --scene … --snap`), in the
page's language and in both themes, and the browser picks the theme. A language not rendered
yet shows the English pictures. Nothing is drawn by hand. The first picture is four captures of one window with Next pressed between them; the
page's own Prev and Next step through them, so a visitor presses the key the program is
about. `tools/site-shots.sh` renders all of them again after a screen changes.

The entrance hides content only after the page's inline script has run, and shows it
regardless after four seconds. A script that never loads, or a tab that never paints,
cannot leave the page blank.

## Words

No language is translated from another. The English is written first as a fact sheet
(each claim is in `README.md` or `docs/features.md`). Every other language is composed by its
own writer, who sees only `brief.json` and the program's own words in that language (owner,
2026-11-12: writers compose every language, then native speakers correct). Change a brief
before changing a string. `tools/build-site.py` stops when a language lacks a key, when
the page asks for a string nobody wrote, or when a string is never shown.

## Checks

| Check | Command | Last result (2026-11-11) |
|---|---|---|
| Banned patterns in the source | `node ~/.claude/skills/site-design/scripts/lint.mjs target/site/index.html site/style.css site/script.js` | 0 fail; the brand's nav line, footer line and glow border are marked as kept |
| The rendered page at 1440, 768 and 390 | `node ~/.claude/skills/site-design/scripts/inspect.mjs http://localhost:47932/ --allow hairline,gradient,button-fx` | 0 fail, 1 warning (no video) in both languages |
| Machine-prose tells in the English | `bun ~/.claude/skills/human-prose/scripts/ai-prose-scan.ts` on the strings | 0 banned, 4 capped lists, all real inventories |

The warning stands: no honest clip of the program exists yet, and the stepped window does
that job. The page has not been opened on a real phone.

## References

The mock the download area was ported from is in `target/site-mock/a/site/`, its pictures in
`~/Shared/inspo/artosia-site-downloads/mocks/` (`a-hero-1440.png`, `a-hero-390.png`, `a-get-1440.png`).
The inspector's last result for the area (2026-11-12): 0 FAIL, 1 WARN (no video) with and without the Mac row.
