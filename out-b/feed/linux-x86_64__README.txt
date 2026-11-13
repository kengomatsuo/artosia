Artosia for Linux
=================

A free worship presentation program: songs, Bible verses and backgrounds on the projector.
https://artosia.matsuokengo.com

Start
-----
Unpack this folder anywhere in your home folder and open "artosia" inside it.
To add Artosia to the applications menu, run ./install.sh once.

Video backgrounds, cameras and streaming
----------------------------------------
These use the GStreamer that your distribution provides. Without it Artosia still shows
songs and verses on a plain or picture background.

Debian and Ubuntu:

    sudo apt-get install gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
        gstreamer1.0-plugins-bad gstreamer1.0-libav

Other distributions: GStreamer lists its packages at
https://gstreamer.freedesktop.org/documentation/installing/on-linux.html
Artosia needs the base, good and bad plugin sets and a decoder for H.264 video.

Updates
-------
Artosia looks for a newer version by itself a few seconds after it starts, fetches it into
this folder and uses it the next time it starts. The folder must stay writable by you, so
keep it in your home folder.

Remove
------
Run ./install.sh --remove, then delete this folder. Your songs and settings are kept apart
from it, in ~/.local/share/artosia.

Licence
-------
GPL-3.0-or-later. Source: https://github.com/kengomatsuo/artosia
