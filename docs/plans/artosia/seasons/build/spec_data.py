"""Hand-written part of spec.json (SEASONMARK1): where each family puts its season mark, how big, in which role, and why.

Every value is a decision of this card. build.py copies it into spec.json; nothing here is read by a drawing except through spec.json.
Sheet = 1920 x 1080. Sizes are px on the sheet. A role is a key of the family's own palette.
Sources (S = ~/Shared/inspo/artosia-templates/seasons/, T = ~/Shared/inspo/artosia-templates/, R = repo docs/plans/artosia/):
  S/yt-4Nzs8kJy2YU.jpg  candle icon after the kicker "MINGGU ADVEN 1" (a 2% sign); S/yt--5cdDw3leRQ.jpg two candles the next week
  S/yt-3frYjkCRTs0.jpg  restrained new-year ornament on cream: sparkle and scallop, data left clean
  S/yt-SaBMHGmaESI.jpg  a 4% flame in place of a letter; S/yt-TXsWxiCvxH0.jpg easter cave, no figure in our mark
  S/cvs-ash-wednesday-ministry-graphics.jpg  the cross alone is the mark; S/yt-FZXgpO9QgOY.jpg crown of thorns behind the title
  S/yt-sRCenena5Kg.jpg  wheat, restrained frame; S/notes.md rows 8, 9, 46, 84, 105, 129, 156, 164 (the finding)
"""

FIXTURE = {
    "church": "Gereja Bethel Kasih Nusantara",
    "kicker_word": "Warta jemaat",
    "title": "Jadwal Pekan Ini",
    "range": "15 sampai 21 November 2026",
    "range_short": "15 - 21 November 2026",
    "rows": [["07.00", 0.46], ["09.30", 0.58], ["18.30", 0.52], ["19.00", 0.40], ["17.30", 0.62], ["19.00", 0.55], ["09.00", 0.48], ["16.00", 0.57]],
    "special_row": 5,
}

# spot kinds: kicker (after the left kicker's measured end), sticker (a round badge in the pill row), ring (inside a neon ring),
# panel (corner of the navy area), cross (replaces the small cross), corner (right corner of the top row), none.
PLACEMENT = {
    1: dict(spot="kicker", size=48, role="accent", ground="paper", cy=96, gap=24, reserve=72, kicker_maxw=[1180, 1108],
            rule="Left of the top line, right after 'Warta jemaat . church' (measured end + 24 px), centred on the kicker's line (y 96).",
            why="The kicker line is the one line every kind of this family draws; a 2% sign beside a caps label is what GKI Bromo does (yt-4Nzs8kJy2YU.jpg).",
            never="Editorial greige lists no icon rule; the sign is a type ornament in the accent."),
    2: dict(spot="sticker", size=48, role="ink", ground="orange", cy=84, disc=72, cx_from_x2=36, reserve=0,
            rule="A round orange sticker 72 px across at the right end of the pill row (centre x2-36, y 84), the mark 48 px in ink inside it.",
            why="Pill poster already speaks in round stickers (the KHUSUS stamp and the DAFTAR disc, d2.rs lines 22 and 105); the mark rides one more, in the top corner nothing else uses.",
            never="No rule against it; the sticker is the family's own shape."),
    3: dict(spot="ring", size=0, ratio=0.5, role="ring colour", ground="night1", rings="first ring of the kind's deco list with a diameter of 200 or less", reserve=0,
            rule="Inside the first neon ring of the kind with a diameter of 200 px or less, centred, 50% of the ring's diameter (S01 88, S04 75, S06 60 px), in the ring's own colour.",
            why="The ring is empty decoration (d3.json deco); a season mark inside it adds nothing to the slide and the ring's colour already passes the 3:1 mark pair on night1.",
            never="No rule against it; the ring is the family's ornament."),
    4: dict(spot="panel", size=56, role="sky", ground="navy", x_from_x2=56, y=40, fallback=dict(spot="kicker", role="blue", ground="light", size=48), reserve=0,
            rule="In the top right corner of the navy area, 56 px, at x2-56 and y 40, in sky. Where the kind has no navy at the top right (verse, notice with a picture) it sits after the kicker in blue, 48 px.",
            why="The navy panel is already a side strip in the season's colour (d4.rs ground()); the mark is a corner sign on that strip, as GKI Bromo puts the pastor circle on its purple strip (yt--5cdDw3leRQ.jpg).",
            never="No rule against it."),
    5: dict(spot="kicker", size=48, role="accent", ground="paper", cy=88, gap=24, reserve=72, kicker_maxw=None,
            rule="After the church name on the top line (measured end + 24 px), centred on y 88; the church name's maxw is lowered by 72.",
            why="Plain ground, one line on top; the quietest place a sign can go (sgo-ash-wednesday.jpg: a small cross on a hairline).",
            never="Simple lists no icon rule."),
    6: dict(spot="kicker", size=48, role="mute", ground="paper", cy=73, gap=24, reserve=72, kicker_maxw=None, exception="no icon",
            rule="After 'Warta jemaat' on the top line (measured end + 24 px), centred on the cap height of the kicker (baseline 84, so y 73); mute, never accent.",
            why="The default look. The mark is set like a fleuron: same colour as the caps label, 9 px clear of the rule at y 106, accent kept for the highlight.",
            never="templates-default.md candidate A says 'no icon'. This card amends it by one clause: no icon except the season mark, in mute, beside the kicker. Alternative if the owner refuses: no mark, the wash alone."),
    7: dict(spot="none", size=0, role="", ground="", reserve=0, fallback_if_overruled="after the footer kicker, mute, 48 px",
            rule="No mark. The colourway (block, accent) carries the season.",
            why="Swiss numerals says no icon, no rounded shape, one accent only for the highlight and the total (templates-default.md candidate B). A round-capped sign breaks all three.",
            never="Kept."),
    8: dict(spot="none", size=0, role="", ground="", reserve=0, fallback_if_overruled="after 'Warta jemaat', mute, 48 px",
            rule="No mark. The tinted ground and the pill carry the season.",
            why="Quiet tint says no icon and no second accent (templates-default.md candidate C); its pill is the only coloured shape.",
            never="Kept."),
    9: dict(spot="none", size=0, role="", ground="", reserve=0, fallback_if_overruled="after the band kicker, band_mute, 48 px",
            rule="No mark. The black band and its accent carry the season.",
            why="Civic board says no icon and no rounded corner (templates-default.md candidate D); a departure-board header takes no pictogram.",
            never="Kept."),
    10: dict(spot="kicker", size=48, role="sky", ground="brand_dk", cy=62, gap=24, reserve=72, kicker_maxw=None,
             rule="After the kicker on the header block (measured end + 24 px), centred on y 62; the header's room() is lowered by 72.",
             why="The kicker sits on the deep block in sky; the sign takes the same ink, so it is already a tested pair (sky on brand_dk).",
             never="No rule against it."),
    11: dict(spot="cross", size=56, role="gold", ground="ground", cx=960, cy=92, reserve=0,
             rule="Replaces the small cross between the two gold rules: centred at (960, 92), 56 px, gold. The rules keep their ends 40 px from the centre, so the 80 px gap holds it.",
             why="The family's only pictogram is that cross (d11.json cross_*); a season swaps it for the season's sign and draws it again when no season applies.",
             never="Nothing amended: the slot already holds a sign."),
    12: dict(spot="kicker", size=48, role="white", ground="scrim", cy=72, gap=24, reserve=72, kicker_maxw=None,
             rule="After the kicker on the photo (measured end + 24 px), centred on y 72, white at the kicker's 90% opacity.",
             why="Words on the photo sit on a scrim; the sign takes the kicker's own white, so it is as safe as the words.",
             never="No rule against it."),
    13: dict(spot="kicker", size=48, role="deep", ground="signal", cy=56, gap=24, reserve=72, kicker_maxw=None,
             rule="After the kicker on the signal band (measured end + 24 px), centred on y 56, in deep.",
             why="Hard blocks, one mark on the band beside the label, in the same near-black the label has.",
             never="No rule against it."),
    14: dict(spot="kicker", size=48, role="accent", ground="g2", cy=84, gap=24, reserve=72, kicker_maxw=None,
             rule="After the kicker (measured end + 24 px), centred on y 84, in accent.",
             why="Accent on g2 is already one of the family's listed mark pairs.",
             never="No rule against it."),
    15: dict(spot="kicker", size=48, role="accent", ground="ground", cy=56, gap=24, reserve=72, kicker_maxw=None,
             rule="After 'Warta jemaat' in the top left folio line (measured end + 24 px), centred on y 56; the church name at the right is left out when it no longer fits, as it already is.",
             why="A magazine folio line carries a small ornament; accent on ground is a listed mark pair.",
             never="No rule against it."),
    16: dict(spot="corner", size=48, role="ink", ground="ground", cy=50, x_from_x2=48, reserve=72,
             rule="At the right end of the top row, right edge at x2, centred on y 50 (the kicker stays at y 38; the chip below starts at y 89); the kicker's maxw is lowered by 72.",
             why="The kicker row is only 54 px under the top edge, so a sign beside it would leave the title-safe area; the right corner above the chip is empty in every kind.",
             never="No rule against it."),
    17: dict(spot="kicker", size=48, role="accent", ground="ground", cy=80, gap=24, reserve=72, kicker_maxw=None,
             rule="After the kicker (measured end + 24 px), centred on y 80; the arches at the right stay behind nothing.",
             why="Warm tone-on-tone with one accent; the arches rise at the right, so the left of the kicker line is free.",
             never="No rule against it."),
}

# Which mark a season draws. Advent's week comes from the date (advent_week below). Harvest and New Year are not seasons of the engine.
SEASON_MARK = {
    "advent": "advent-N",
    "christmas": "christmas",
    "epiphany": "epiphany",
    "lent": "lent",
    "holy-week": "palm",
    "good-friday": "good-friday",
    "easter": "easter",
    "ascension": "ascension",
    "pentecost": "pentecost",
    "reformation": "reformation",
    "ordinary": None,
}
BY_HAND_ONLY = ["harvest", "new-year"]

ADVENT_WEEK_RULE = "week n = 1 + whole weeks from the first Sunday of Advent to today (1 to 4); the fourth week runs to 24 December; a card in the advent colourway outside Advent draws the single candle, advent.svg"


# What already sits in design B's top right rectangle (x2-160 to x2, y 40 to 200), read from the renderers in crates/artosia-slides/src/warta/dN.rs.
TOP_RIGHT = {
    1: "the right kicker (kicker_r) at y 96 and the title row's date at its right",
    2: "nothing in the pill row; a long title runs under it",
    3: "the neon ring, diamond and dot (deco)",
    4: "the day cells of the top band (S02) and the navy panel's list (S01)",
    5: "the right ribbon words at y 88",
    6: "the church name at the right of the kicker line and its rule",
    7: "the date range at the right of the title",
    8: "the church name at the right of the kicker line",
    9: "the right kicker in the black band and the date range under it",
    10: "the date range chip with its arrow (chip_cy 168)",
    11: "nothing: the frame's corner is empty",
    12: "the sub line at the right (sub_cy 120)",
    13: "the small line on the deep block (kicker_cy 56)",
    14: "the date pill (pill_cy 150)",
    15: "the church name in the right folio line",
    16: "the tag chip (tag_cy 120)",
    17: "the right kicker and the arches",
}
TOP_RIGHT_FREE = [2, 11]
