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

wrangler is signed in on the Mac and not on this PC, so `tools/publish-site.sh` copies the
built folder to the Mac over WSL's SSH and runs `wrangler deploy` there. When the Mac cannot
be reached the script says so and the site keeps its last deploy; the page's own script
still shows the newest version and sizes, read from GitHub when it is opened.

GitHub Pages served the first copy for a day (2026-11-11) from a `gh-pages` branch, waiting
for a DNS record at Cloudflare. It was switched off and the branch deleted on 2026-11-12,
so one copy exists.

`tools/publish.sh` ends by running `tools/publish-site.sh`, so each release rewrites the
version and size on the page. The script reads both from the newest release, never from
`Cargo.toml`, so the page cannot name a version nobody can download. The page's own script
then checks the newest release when it is opened and keeps the answer for an hour.

The download button points at
`https://github.com/kengomatsuo/artosia/releases/latest/download/Artosia-setup.exe`,
which always answers with the newest release's setup ([setup.md](setup.md)). The zip of the
same release is the second way, for a computer with no internet, under the three steps.

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
