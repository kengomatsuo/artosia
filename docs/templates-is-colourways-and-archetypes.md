# Announcement templates are colourways over layout families (decided 2026-11-15)

Directed by the owner's ask for a Canva-like library ("hundreds of templates with different approaches") and a default that is already good. Do not relitigate without new evidence.

## The decision: copy the design, build the code

No candidate can be the engine. FreeShow and OpenLP are GPL apps in other languages; Office, Keynote, Google Slides, ProPresenter and Canva are formats or products; and none of their placeholder models holds a list of unknown length that shrinks to a floor and then continues on a second slide, which every Artosia data slide needs. The research, with every source saved, is `~/Shared/inspo/artosia-templates/prior-art/README.md`.

Artosia copies the split PowerPoint, Keynote and Google Slides share:

| Part | From | In Artosia |
|---|---|---|
| Layout family | slide masters and layouts | a warta direction: one Rust file drawing the six kinds (`crates/artosia-slides/src/warta/dN.rs`), its tokens in `assets/warta/tokens.json` |
| Colour scheme by role | OOXML `clrScheme` (12 slots), Google's 12 theme colour types, Canva's colour themes | `colourways` per direction: each a full map of the direction's colour keys, the design's own first |
| Font scheme | OOXML major and minor fonts, Canva's H1 to H5 and body | the direction's `fonts`; a later step may add font pairs per colourway |
| Contrast | WCAG 2.2 1.4.3, 1.4.6, 1.4.11; Radix and Material solve text steps for contrast | a test: every text-on-ground pair a direction lists passes 4.5:1 (3:1 for large text and marks) in every colourway |

A template is a family and a colourway, stored in the card's style column as `family/colourway` (a plain family code is its first colourway, so old libraries read unchanged). Many templates cost JSON; a new approach costs one layout file, reusable by every colourway after it.

## Rejected

- Merging content into a template by item order (FreeShow): one inserted item shifts every later one. Artosia fills by named part (`Field`).
- One template per Rust file with fixed colours (Artosia until now): nine styles cost nine files and gave nine looks.
- A free colour picker for template colours: Canva curates colour themes rather than letting any pair through; a hall reads from a distance, so only gated colourways ship.

## Not built yet

- Shared colour and type role names across families, so one archetype serves several families (the README's step 1 and 3). Families keep their own key names until an archetype is shared.
- Font pairs per colourway.
- Generated colourway candidates (oklch tone steps on one accent, then the contrast gate, then a person picks).
