"""Works out, and copies, the part of an installed GStreamer that Artosia needs.

    python tools/gstreamer-subset.py                      # list the files and their size
    python tools/gstreamer-subset.py --copy <folder>      # copy them as <folder>/bin and <folder>/lib/gstreamer-1.0

The plugins are named below, by what Artosia asks of them. Everything else follows from
them: each plugin's imports are read from the file itself and followed through GStreamer's
bin folder until nothing new turns up. Windows' own DLLs are not in that folder and are left out.
"""
import os, shutil, struct, sys, zipfile

ROOT = os.environ.get("GSTREAMER_1_0_ROOT_MSVC_X86_64", r"C:\Program Files\gstreamer\1.0\msvc_x86_64")
BIN = os.path.join(ROOT, "bin")
PLUGINS = os.path.join(ROOT, "lib", "gstreamer-1.0")

# plugin file name without "gst" and ".dll": why Artosia needs it
VIDEO = {
    "coreelements": "queue, capsfilter, filesrc, filesink, fakesink, typefind",
    "app": "appsink, which hands frames to the projector",
    "playback": "playbin3, which builds the video background's pipeline",
    "typefindfunctions": "telling an mp4 from a mov or an mkv",
    "isomp4": "mp4 and mov files; mp4mux for a recording",
    "matroska": "mkv and webm files",
    "videoparsersbad": "h264parse and its siblings",
    "audioparsers": "aacparse; a video file's sound track is parsed even when muted",
    "d3d11": "hardware decoding, and the stream's screen capture and colour conversion",
    "d3d12": "hardware decoding on newer drivers",
    "mediafoundation": "cameras, and Windows' own H.264 and AAC encoders",
    "libav": "software decoding where the graphics chip cannot, and an AAC encoder",
    "videoconvertscale": "videoconvert, for a camera's own colour format",
    "jpeg": "cameras that send JPEG frames",
    "vpx": "webm video",
    "dav1d": "AV1 video",
    "pbtypes": "types the playback elements share",
    "volume": "playbin3 builds a volume control even when muted",
    "audioconvert": "sound format conversion",
    "audioresample": "sound rate conversion",
    # A camera on the network, a phone's camera app among them (srt itself is in STREAM).
    "mpegtsdemux": "the MPEG-TS that an SRT camera sends",
    "rtsp": "rtspsrc, for a camera at an rtsp:// address",
    "rtpmanager": "the jitter buffer an RTSP camera needs",
    "rtp": "unpacking H.264, H.265 and JPEG from RTP",
    "udp": "RTSP's packets",
    "tcp": "RTSP over TCP, and a plain tcp:// camera",
    "soup": "a camera at an http:// address",
    "multipart": "the JPEG stream a phone's camera app serves over HTTP",
}
STREAM = {
    "wasapi2": "the room's sound from a microphone or the PC",
    "videotestsrc": "the stream program's self-test",
    "audiotestsrc": "the stream program's self-test",
    "flv": "flvmux, for RTMP",
    "rtmp2": "rtmp2sink",
    "mpegtsmux": "mpegtsmux, for SRT",
    "srt": "srtsink",
    "x264": "software H.264 encoding where no hardware encoder exists",
    "nvcodec": "Nvidia's encoder",
    "qsv": "Intel's encoder",
    "amfcodec": "AMD's encoder",
}


def imports(path):
    """The DLL names a Windows program file imports, read from its import tables."""
    with open(path, "rb") as f:
        data = f.read()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        return []
    sections = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    plus = struct.unpack_from("<H", data, optional)[0] == 0x20B
    directories = optional + (112 if plus else 96)
    table = optional + optional_size
    rows = [struct.unpack_from("<IIII", data, table + 40 * i + 8) for i in range(sections)]  # vsize, vaddr, rawsize, rawptr

    def offset(rva):
        for vsize, vaddr, rawsize, rawptr in rows:
            if vaddr <= rva < vaddr + max(vsize, rawsize):
                return rva - vaddr + rawptr
        return None

    def name_at(rva):
        at = offset(rva)
        if at is None:
            return None
        end = data.index(b"\0", at)
        return data[at:end].decode("ascii", "replace")

    found = []
    # 1: the import table (20-byte rows, name at +12). 13: delay-loaded imports (32-byte rows, name at +4).
    for index, size, name_field in ((1, 20, 12), (13, 32, 4)):
        rva = struct.unpack_from("<I", data, directories + 8 * index)[0]
        at = offset(rva) if rva else None
        while at is not None:
            row = data[at:at + size]
            if len(row) < size or not any(row):
                break
            name = name_at(struct.unpack_from("<I", row, name_field)[0])
            if name:
                found.append(name)
            at += size
    return found


def closure(plugins):
    in_bin = {name.lower(): name for name in os.listdir(BIN) if name.lower().endswith(".dll")}
    plugin_files, missing = [], []
    for plugin in plugins:
        name = f"gst{plugin}.dll"
        (plugin_files if os.path.isfile(os.path.join(PLUGINS, name)) else missing).append(name)
    need, queue = {}, [os.path.join(PLUGINS, name) for name in plugin_files]
    # Programs GStreamer starts by itself: the plugin scanner keeps a bad plugin from taking the app down.
    for helper in ("gst-plugin-scanner.exe",):
        for folder in (os.path.join(ROOT, "libexec", "gstreamer-1.0"), BIN):
            if os.path.isfile(os.path.join(folder, helper)):
                queue.append(os.path.join(folder, helper))
                break
    seen_helpers = [path for path in queue if path.lower().endswith(".exe")]
    while queue:
        for name in imports(queue.pop()):
            real = in_bin.get(name.lower())
            if real and real not in need:
                need[real] = os.path.join(BIN, real)
                queue.append(need[real])
    return plugin_files, need, missing, seen_helpers


def megabytes(paths):
    return sum(os.path.getsize(path) for path in paths) / 1e6


def main():
    copy_to = sys.argv[sys.argv.index("--copy") + 1] if "--copy" in sys.argv else None
    video_only = "--video-only" in sys.argv
    wanted = dict(VIDEO) if video_only else {**VIDEO, **STREAM}
    plugin_files, libraries, missing, helpers = closure(wanted)
    plugin_paths = [os.path.join(PLUGINS, name) for name in plugin_files]
    print(f"{len(plugin_files)} plugins, {megabytes(plugin_paths):.1f} MB; {len(libraries)} libraries, {megabytes(libraries.values()):.1f} MB; "
          f"{len(helpers)} helper, {megabytes(helpers):.1f} MB; together {megabytes(plugin_paths) + megabytes(libraries.values()) + megabytes(helpers):.1f} MB")
    if missing:
        print("not in this GStreamer:", ", ".join(missing))
    if "--list" in sys.argv:
        for path in sorted(list(libraries.values()) + plugin_paths, key=os.path.getsize, reverse=True)[:25]:
            print(f"  {os.path.getsize(path) / 1e6:6.1f} MB  {os.path.basename(path)}")
    if copy_to:
        for folder, paths in ((os.path.join(copy_to, "bin"), list(libraries.values())), (os.path.join(copy_to, "lib", "gstreamer-1.0"), plugin_paths),
                              (os.path.join(copy_to, "libexec", "gstreamer-1.0"), helpers)):
            os.makedirs(folder, exist_ok=True)
            for path in paths:
                shutil.copy2(path, os.path.join(folder, os.path.basename(path)))
        # The licences of everything GStreamer's installer ships, in one file beside them.
        licences = os.path.join(ROOT, "share", "licenses")
        if os.path.isdir(licences):
            with zipfile.ZipFile(os.path.join(copy_to, "licenses.zip"), "w", zipfile.ZIP_DEFLATED) as packed:
                for folder, _, names in os.walk(licences):
                    for name in sorted(names):
                        path = os.path.join(folder, name)
                        packed.write(path, os.path.relpath(path, licences))
        print("copied to", copy_to)


if __name__ == "__main__":
    main()
