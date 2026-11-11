# Artosia board

## Backlog
- **INSTALL1** A one-click installer for Windows, since the download is a zip to unpack · source: session 2026-11-11

## Ready

## Doing (limit 1 per session)
- **REMOTE2** The phone remote redone from Mobbin and real remotes, directions shown as pictures for the owner to pick · source: owner 2026-11-11 · plan: four looks under crates/artosia-remote/examples/directions, one per real remote; the owner picks from the contact sheet; src/page.html is then ported from the picked file · done when: the owner has picked, page.html is the port of that look with every function kept and Indonesian written from briefs · verify: cargo test -p artosia-remote; the page shot at 390 and 320 against the picked look · claim: main 2026-11-11T16:55:23Z · contact sheet sent 2026-11-11; waiting for the owner's pick

## Needs review
- **SITE1** A homepage at artosia.matsuokengo.com in the matsuokengo.com look, with the one file to download · source: owner 2026-11-11 · plan: web/ static page ported from the brand's saved code, real renders as pictures, published to gh-pages with CNAME · done when: the page serves from GitHub Pages with the custom domain set and the download button fetches the zip · verify: site-design lint.mjs and inspect.mjs 0 FAIL at 1440, 768, 390; curl of the Pages URL returns 200 · DNS record for artosia.matsuokengo.com is the owner's to add at Cloudflare; not opened on a real phone · commit: 86edc0c · check: site-design lint 0 fail; inspect 0 fail at 1440, 768, 390 in both languages; GitHub's server answers 200 for /, /id/ and a picture under the custom name

## Done
