"""Sends one slide to a projector program started on a spare port, as the operator would."""
import json, socket, sys, time

port = int(sys.argv[1])
text = "A line of made-up words for the test\nA second line to check the width"
state = {
    "slide": {"kind": "Lyrics", "label": "Verse 1", "short": "V1", "tag": {"kind": "Verse", "number": 1}, "heading": "", "text": text, "second": "", "caption": "", "image": ""},
    "look": {"name": "Midnight", "background": {"Gradient": [[16, 24, 48], [6, 8, 18]]}, "ink": [255, 255, 255], "size": 1.0, "weight": 700.0, "centered": True, "place": "Middle", "shadow": False, "dim": 0.35, "max_lines": 4},
    "black": False, "text_hidden": False, "logo": "", "logo_on": False, "message": "", "countdown": None,
    "stage": {"title": "", "next": None, "next_title": "", "index": 0, "count": 0, "chips": [], "message": ""},
}
# A third argument "languages" sends a song's slide in three languages, each a block of made-up lines.
if len(sys.argv) > 3 and sys.argv[3] == "languages":
    state["slide"].update({
        "text": "Baris contoh pertama\nBaris contoh kedua\nBaris contoh ketiga\nBaris contoh keempat",
        "lang": "id",
        "slot": 0,
        "more": [
            {"lang": "en", "slot": 1, "text": "A first made-up line\nA second made-up line\nA third made-up line\nA fourth made-up line"},
            {"lang": "zh-Hans", "slot": 2, "text": "第一行示例文字\n第二行示例文字\n第三行示例文字\n第四行示例文字"},
        ],
    })
for attempt in range(40):
    try:
        s = socket.create_connection(("127.0.0.1", port), timeout=1)
        break
    except OSError:
        time.sleep(0.25)
else:
    sys.exit("the projector never listened")
s.sendall((json.dumps({"Place": {"audience": None, "stage": None, "stage_window": False, "here": False, "off": False}}) + "\n").encode())
s.sendall((json.dumps({"State": state}) + "\n").encode())
time.sleep(float(sys.argv[2]) if len(sys.argv) > 2 else 6)
s.sendall(b'"Quit"\n')
time.sleep(0.5)
