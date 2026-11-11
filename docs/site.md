# The homepage

`artosia.matsuokengo.com` is one static page in English and one in Indonesian. Its job is
the one the owner named on 2026-11-11: a volunteer finds the single file to download and
knows what to do with it.

## Where it lives

| Thing | Place |
|---|---|
| The page, once, with `{{key}}` holes | `site/page.html` |
| Its words | `site/strings/en.json`, `site/strings/id.json` |
| What each Indonesian string must do | `site/strings/brief.json` |
| Look and motion | `site/style.css`, `site/script.js` |
| Pictures | `site/img/`, made by `tools/site-shots.sh` |
| The built site | `target/site/`, made by `tools/build-site.py` |
| The served copy | branch `gh-pages` of `kengomatsuo/artosia`, replaced by `tools/publish-site.sh` |

## How it is served

GitHub Pages serves the `gh-pages` branch at the address in its `CNAME` file. The owner's
other product pages (`cutling.`, `undirect.`) work the same way. The domain's DNS is at
Cloudflare and needs one record: type CNAME, name `artosia`, target `kengomatsuo.github.io`,
proxy off (DNS only), so GitHub can issue the certificate. Once the name resolves:
`gh api -X PUT repos/kengomatsuo/artosia/pages -F https_enforced=true`.

`tools/publish.sh` ends by running `tools/publish-site.sh`, so each release rewrites the
version and size on the page. The script reads both from the newest release, never from
`Cargo.toml`, so the page cannot name a version nobody can download. The page's own script
then checks the newest release when it is opened and keeps the answer for an hour.

The download button points at
`https://github.com/kengomatsuo/artosia/releases/latest/download/Artosia-setup.exe`,
which always answers with the newest release's setup ([setup.md](setup.md)). The zip of the
same release is the second way, for a computer with no internet, under the three steps.

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
page's language and in both themes, and the browser picks the theme. Nothing is drawn by
hand. The first picture is four captures of one window with Next pressed between them; the
page's own Prev and Next step through them, so a visitor presses the key the program is
about. `tools/site-shots.sh` renders all of them again after a screen changes.

The entrance hides content only after the page's inline script has run, and shows it
regardless after four seconds. A script that never loads, or a tab that never paints,
cannot leave the page blank.

## Words

No language is translated from the other. The English is written first as a fact sheet
(each claim is in `README.md` or `docs/features.md`). The Indonesian is composed by a
writer who sees only `brief.json` and the program's own Indonesian words. Change a brief
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
