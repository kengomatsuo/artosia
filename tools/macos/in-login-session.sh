#!/usr/bin/env bash
# Runs a script where the login keychain is open:
#
#   tools/macos/in-login-session.sh <script> [arguments]
#
# A shell that came in over SSH cannot use the keychain: `codesign` ends with
# errSecInternalComponent and `notarytool` with keychainLocked (both read on 2026-11-13), and
# opening it would take the Mac's password, which is typed by nobody but its owner. A Terminal
# of the person who is signed in has it open, so the script is handed to Terminal as a
# .command file and this shell waits for it. Run from such a Terminal, the script runs here.
set -euo pipefail

script="${1:?Give the script}"; shift
if security show-keychain-info "$HOME/Library/Keychains/login.keychain-db" > /dev/null 2>&1; then
    exec bash "$script" "$@"
fi

pgrep -x loginwindow > /dev/null || { echo "Nobody is signed in at this Mac, so its keychain is closed."; exit 1; }
work="$(mktemp -d "${TMPDIR:-/tmp}/artosia-login.XXXXXX")"
run="$work/Artosia signing.command"
{
    echo '#!/bin/bash'
    printf 'cd %q\n' "$PWD"
    printf 'export PATH=%q\n' "$PATH"
    for name in ARTOSIA_MAC_IDENTITY ARTOSIA_NOTARY_PROFILE; do
        [ -n "${!name:-}" ] && printf 'export %s=%q\n' "$name" "${!name}"
    done
    printf 'bash %q' "$script"; printf ' %q' "$@"; printf ' > %q 2>&1\n' "$work/log"
    printf 'echo $? > %q\n' "$work/done"
} > "$run"
chmod +x "$run"
open -g -a Terminal "$run"
waited=0
while [ ! -f "$work/done" ]; do
    sleep 5; waited=$((waited + 5))
    [ $waited -lt 6000 ] || { echo "The signing did not finish in 100 minutes."; cat "$work/log" 2>/dev/null | tail -20; exit 1; }
done
cat "$work/log"
code="$(cat "$work/done")"
rm -rf "$work"
exit "$code"
