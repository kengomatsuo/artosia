"""Builds crates/artosia-core/assets/backdrops.json, the catalogue of ready backdrops (docs/backdrops.md).

    python tools/backdrops.py [--cache <folder>] [--out <file>] [--pictures N] [--videos N]

Source: Wikimedia Commons, through its own API (https://commons.wikimedia.org/w/api.php).
A file is listed only when the API's own answer for that file says CC0 or public domain,
names an author, and carries no usage restriction (trademark, personality rights and so on).
Nothing else is decided from a search snippet. No media is downloaded except each picture's
small preview, which is read once to compute a brightness and a busyness hint (needs Pillow;
without it those two fields are left out). The folder given to --cache keeps every answer, so
a second run asks the servers for nothing it already has.

Files a person looked at and turned down are listed in tools/backdrops-exclude.txt, one catalogue
id per line (`wc-` and the Commons page id), then a tab and the reason. Lines starting with # are comments.
"""
import argparse, html, io, json, os, re, sys, time, urllib.error, urllib.parse, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
API = "https://commons.wikimedia.org/w/api.php"
UA = "ArtosiaBackdropCatalogue/1.0 (free GPL worship presentation app; catalogue builder)"
PAUSE = 2.0  # seconds between two calls; Wikimedia asks API clients to go one at a time

# id, English name (the app names the categories itself), picture searches, video searches
CATEGORIES = [
    ("sky", ["cloudscape", "blue sky white clouds", "sunrise sky", "sunset sky clouds", "dramatic sky storm clouds", "rainbow sky", "cumulus clouds", "golden clouds"],
            ["clouds timelapse", "cloud time-lapse sky", "sunset timelapse", "sunrise timelapse", "clouds", "cloud", "sunset", "sky"]),
    ("mountains", ["mountain landscape", "alpine peaks snow", "mountain lake reflection", "misty mountains", "mountain valley green", "mountain range sunrise"],
                  ["mountain timelapse", "mountains clouds moving", "mountain", "snow mountain", "alps"]),
    ("water", ["ocean waves", "sea horizon calm", "waterfall", "calm lake", "river flowing", "beach shore waves", "lake sunset reflection"],
              ["ocean waves video", "waterfall video", "river flowing water", "waves beach video", "calm lake water ripples", "sea", "river", "lake", "waves", "water"]),
    ("nature", ["forest path", "green meadow", "wheat field", "autumn forest", "flower field", "rural landscape fields", "tree sunset silhouette", "misty forest morning"],
               ["forest video", "grass wind field", "leaves wind trees", "field wheat wind", "flowers wind", "grass", "trees", "forest", "flowers", "leaves", "field"]),
    ("light", ["sun rays through clouds", "crepuscular rays", "sunbeams forest", "golden hour landscape", "light rays fog", "sun burst sky"],
              ["sun rays video", "light rays", "sunlight trees video", "sunbeams", "sunlight", "sun", "light", "lens flare"]),
    ("night", ["milky way", "starry night sky", "aurora borealis", "night sky stars", "full moon", "star trails"],
              ["stars timelapse", "milky way timelapse", "aurora timelapse", "moon time-lapse", "stars", "night sky", "moon", "aurora", "milky way"]),
    ("candles", ["candle flame", "candles dark background", "bokeh lights warm", "lantern light", "fire flames dark", "oil lamp"],
                ["candle flame video", "fire flames video", "fireplace", "bonfire", "lantern video", "candle", "fire", "flame", "lantern"]),
    ("church", ["cross at sunset", "church interior", "stained glass window", "cathedral nave", "chapel", "wooden cross", "church window light", "open bible", "dove"],
               ["cross video", "church video", "stained glass video", "church", "cross", "cathedral", "bells"]),
    ("people", ["praying hands", "hands raised silhouette", "hands holding light", "silhouette sunset hands"],
               ["praying hands video", "hands raised"]),
    ("texture", ["abstract background blur", "soft gradient background", "watercolor texture", "bokeh background", "paper texture", "blurred lights background", "marble texture", "abstract light"],
                ["abstract background video", "bokeh video", "particles video", "abstract motion background", "abstract", "bokeh", "particles", "gradient", "loop"]),
]

ACCEPTED_LICENCE = re.compile(r"^(CC0|Public domain|PD[- ])", re.I)
PICTURE_MIME = {"image/jpeg"}
MIN_WIDTH, MIN_RATIO = 1920, 1.3
VIDEO_SECONDS = (4.0, 90.0)
VIDEO_MIN_WIDTH = 848
PREVIEW_WIDTH = 500  # one of Wikimedia's standard thumbnail widths; the window draws it as a tile and as a large preview


def text_of(markup):
    """HTML from the API turned into the plain text a person would read."""
    return re.sub(r"\s+", " ", html.unescape(re.sub(r"<[^>]+>", "", markup or ""))).strip()


class Client:
    def __init__(self, cache):
        self.cache = cache
        os.makedirs(cache, exist_ok=True)
        self.last = 0.0

    def _wait(self):
        gap = PAUSE - (time.time() - self.last)
        if gap > 0:
            time.sleep(gap)
        self.last = time.time()

    def fetch(self, url, name):
        path = os.path.join(self.cache, name)
        if os.path.isfile(path):
            with open(path, "rb") as f:
                return f.read()
        for attempt in range(6):
            self._wait()
            try:
                request = urllib.request.Request(url, headers={"User-Agent": UA})
                with urllib.request.urlopen(request, timeout=60) as response:
                    data = response.read()
                with open(path, "wb") as f:
                    f.write(data)
                return data
            except urllib.error.HTTPError as error:
                if error.code in (429, 500, 502, 503, 504):
                    wait = int(error.headers.get("Retry-After") or 10 * (attempt + 1))
                    print(f"  {error.code}, waiting {wait}s", file=sys.stderr)
                    time.sleep(wait)
                    continue
                if error.code == 404:
                    return None
                raise
        raise RuntimeError(f"gave up on {url}")

    def api(self, params, name):
        query = urllib.parse.urlencode({"format": "json", "formatversion": "2", **params})
        data = self.fetch(f"{API}?{query}", name + ".json")
        return json.loads(data)


def slug(text):
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


def load_excluded():
    path = os.path.join(HERE, "backdrops-exclude.txt")
    ids = set()
    if os.path.isfile(path):
        for line in open(path, encoding="utf-8"):
            line = line.split("\t")[0].strip()
            if line and not line.startswith("#"):
                ids.add(line)
    return ids


def search_pages(client, search, kind, width, tag):
    """One page of search results with the facts the catalogue needs. `width` is the thumbnail asked for."""
    props = "imageinfo|videoinfo" if kind == "video" else "imageinfo"
    params = {
        "action": "query", "generator": "search", "gsrnamespace": "6", "gsrlimit": "50", "gsrsearch": search,
        "prop": props, "iiprop": "url|size|mime|mediatype|extmetadata", "iiurlwidth": str(width),
        "iiextmetadatafilter": "LicenseShortName|LicenseUrl|Artist|Restrictions|ObjectName|ImageDescription|Categories",
    }
    if kind == "video":
        params["viprop"] = "derivatives|url|size|mime"
    return client.api(params, f"{tag}-{width}").get("query", {}).get("pages", [])


def licence_of(meta):
    short = text_of(meta.get("LicenseShortName", {}).get("value"))
    if not ACCEPTED_LICENCE.match(short):
        return None
    if text_of(meta.get("Restrictions", {}).get("value")):
        return None
    author = text_of(meta.get("Artist", {}).get("value"))
    if not author or len(author) > 120:
        return None
    url = text_of(meta.get("LicenseUrl", {}).get("value")).replace("http://", "https://", 1)
    if not url:
        url = "https://creativecommons.org/publicdomain/zero/1.0/" if short.upper().startswith("CC0") else "https://commons.wikimedia.org/wiki/Commons:Copyright_tags#Public_domain"
    return short, url, author


def title_of(page, meta):
    name = text_of(meta.get("ObjectName", {}).get("value")) or re.sub(r"\.[a-z0-9]+$", "", page["title"][5:], flags=re.I)
    return re.split(r"label QS:", name)[0].strip()[:90]


def pick_video_file(info):
    """The file to fetch for a video: the smallest VP9 transcode of 720 lines or more, else the
    largest smaller one (480 lines at least), else the original when it is modest."""
    options = [item for item in info.get("derivatives", []) if "vp9" in item.get("type", "") and item.get("height", 0) >= 480]
    big = [item for item in options if item["height"] >= 720]
    chosen = min(big, key=lambda item: item["height"]) if big else (max(options, key=lambda item: item["height"]) if options else None)
    if chosen:
        return chosen["src"], chosen["width"], chosen["height"], chosen.get("bandwidth", 0)
    if info.get("mime") == "video/webm" and info.get("height", 0) <= 1080 and info.get("size", 0) <= 40_000_000:
        return info["url"], info["width"], info["height"], 0
    return None


def clean_url(url):
    return url.split("?utm_")[0] if "?utm_" in url else url


def collect(client, excluded, per_query, want_kind):
    found, seen = [], set()
    for category, pictures, videos in CATEGORIES:
        for search in (pictures if want_kind == "picture" else videos):
            tag = f"{want_kind}-{category}-{slug(search)}"
            filt = "filetype:bitmap filew:>1919" if want_kind == "picture" else "filetype:video"
            query = f"{search} incategory:CC-zero {filt}"
            try:
                big = search_pages(client, query, want_kind, 1920 if want_kind == "picture" else PREVIEW_WIDTH, tag)
                small = {page["title"]: page for page in search_pages(client, query, want_kind, PREVIEW_WIDTH, tag)} if want_kind == "picture" else {}
            except RuntimeError as error:
                print(f"  skipped {search!r}: {error}", file=sys.stderr)
                continue
            took = 0
            for page in sorted(big, key=lambda page: page.get("index", 0)):
                if took >= per_query or page["title"] in seen:
                    continue
                info = (page.get("imageinfo") or [{}])[0]
                meta = info.get("extmetadata", {})
                licence = licence_of(meta)
                if not licence:
                    continue
                short, licence_url, author = licence
                entry = {"id": f"wc-{page['pageid']}", "title": title_of(page, meta), "kind": want_kind, "category": category,
                         "author": author, "licence": short, "licence_url": licence_url, "page": clean_url(info["descriptionurl"]),
                         "source": "wikimedia", "_title": page["title"]}
                if want_kind == "picture":
                    width, height = info.get("width", 0), info.get("height", 0)
                    if info.get("mime") not in PICTURE_MIME or width < MIN_WIDTH or height < 1080 or width / height < MIN_RATIO:
                        continue
                    if "thumburl" not in info:
                        continue
                    preview = small.get(page["title"], {}).get("imageinfo", [{}])[0].get("thumburl")
                    if not preview:
                        continue
                    entry.update(width=width, height=height, url=clean_url(info["thumburl"]), preview=clean_url(preview))
                    if width > 1920:  # the 1920 wide copy is what a volunteer downloads
                        entry["width"], entry["height"] = 1920, round(1920 * height / width)
                else:
                    video = (page.get("videoinfo") or [{}])[0]
                    duration = video.get("duration") or info.get("duration") or 0
                    if not (VIDEO_SECONDS[0] <= duration <= VIDEO_SECONDS[1]) or video.get("width", 0) < VIDEO_MIN_WIDTH:
                        continue
                    chosen = pick_video_file(video)
                    if not chosen or "thumburl" not in info:
                        continue
                    url, width, height, bandwidth = chosen
                    if width / height < 1.3 or width < VIDEO_MIN_WIDTH:
                        continue
                    megabytes = (bandwidth / 8 * duration / 1e6) if bandwidth else video.get("size", 0) / 1e6
                    if megabytes > 45:
                        continue
                    entry.update(width=width, height=height, duration=round(duration, 1), url=clean_url(url), preview=clean_url(info["thumburl"]), megabytes=round(megabytes, 1))
                seen.add(page["title"])
                found.append(entry)
                took += 1
            print(f"{want_kind:8} {category:10} {search:34} {took:3}", file=sys.stderr)
    return found


def hints(client, entries):
    """Brightness and busyness from the small preview: 0 to 100 each."""
    try:
        from PIL import Image, ImageFilter
    except ImportError:
        print("Pillow is missing: brightness and busyness left out", file=sys.stderr)
        return
    for entry in entries:
        try:
            data = client.fetch(entry["preview"], "preview-" + entry["id"] + ".jpg")
        except RuntimeError:
            data = None
        if not data:
            entry["_nopreview"] = True  # a backdrop whose preview cannot be fetched is not listed
            continue
        try:
            picture = Image.open(io.BytesIO(data)).convert("L")
        except Exception:
            continue
        flat = getattr(Image.Image, "get_flattened_data", None) or Image.Image.getdata
        pixels = list(flat(picture))
        entry["brightness"] = round(sum(pixels) / len(pixels) / 255 * 100)
        edges = list(flat(picture.filter(ImageFilter.FIND_EDGES)))
        entry["busy"] = min(100, round(sum(edges) / len(edges) / 255 * 100 * 4))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", default=os.path.join(os.environ.get("TEMP", "."), "artosia-backdrops-cache"))
    parser.add_argument("--out", default=os.path.join(HERE, "..", "crates", "artosia-core", "assets", "backdrops.json"))
    parser.add_argument("--pictures", type=int, default=9, help="most pictures taken from one search")
    parser.add_argument("--videos", type=int, default=4, help="most videos taken from one search")
    args = parser.parse_args()
    client = Client(args.cache)
    excluded = load_excluded()
    entries = collect(client, excluded, args.pictures, "picture") + collect(client, excluded, args.videos, "video")
    # A turned-down file still takes its place in a search's quota, so a new run does not pull in
    # next-ranked files nobody has looked at. Only what the search returned afresh needs a look.
    before = len(entries)
    entries = [entry for entry in entries if entry["id"] not in excluded]
    print(f"{before - len(entries)} of {before} left out by tools/backdrops-exclude.txt", file=sys.stderr)
    hints(client, [entry for entry in entries])
    entries = [entry for entry in entries if not entry.get("_nopreview")]
    for entry in entries:
        entry.pop("_title", None)
    order = {name: at for at, (name, _, _) in enumerate(CATEGORIES)}
    entries.sort(key=lambda entry: (order[entry["category"]], entry["kind"] != "picture", entry["title"].lower()))
    with open(args.out, "w", encoding="utf-8", newline="\n") as f:
        json.dump({"built": time.strftime("%Y-%m-%d"), "source": "Wikimedia Commons", "entries": entries}, f, ensure_ascii=False, indent=1)
        f.write("\n")
    pictures = sum(1 for entry in entries if entry["kind"] == "picture")
    print(f"{len(entries)} entries: {pictures} pictures, {len(entries) - pictures} videos -> {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
