#!/usr/bin/env bash
# Packs a release's programs folder as the Microsoft Store edition (docs/store.md):
#   tools/store-package.sh <version>
# Makes dist/<version>/store/ (the loose folder: register it with Add-AppxPackage -Register) and
# dist/<version>/Artosia-<version>-x64.msix. STORE_DIST=<folder> replaces dist/ (for trials).
set -euo pipefail

version="${1:-}"
[ -n "$version" ] || { echo "usage: tools/store-package.sh <version>" >&2; exit 2; }
repo="$(cd "$(dirname "$0")/.." && pwd)"
dist="${STORE_DIST:-$repo/dist}"
programs="$dist/$version/programs"
[ -d "$programs" ] || { echo "ERROR: $programs is missing; build and release $version first" >&2; exit 1; }

kit="/c/Program Files (x86)/Windows Kits/10/bin/10.0.22621.0/x64"
makeappx="$kit/makeappx.exe"
[ -x "$makeappx" ] || { echo "ERROR: $makeappx not found; install Windows SDK 10.0.22621" >&2; exit 1; }

# The package version is (a+1).b.c.0: Microsoft forbids a first part of 0 and reserves the
# fourth. The same rule as `store::package_version` in crates/artosia/src/store.rs.
case "$version" in
  [0-9]*.[0-9]*.[0-9]*) ;;
  *) echo "ERROR: version must be a.b.c, not $version" >&2; exit 2 ;;
esac
IFS=. read -r major minor patch extra <<<"$version"
[ -z "$extra" ] || { echo "ERROR: version must be a.b.c, not $version" >&2; exit 2; }
package_version="$((major + 1)).$minor.$patch.0"

identity="$repo/tools/store/identity.json"
field() { python -I -c 'import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' "$identity" "$1"; }
name="$(field Name)"; publisher="$(field Publisher)"; display="$(field PublisherDisplayName)"
xml() { local v="$1"; v="${v//&/&amp;}"; v="${v//</&lt;}"; v="${v//>/&gt;}"; echo "${v//\"/&quot;}"; }

loose="$dist/$version/store"
msix="$dist/$version/Artosia-$version-x64.msix"
rm -rf "$loose"; mkdir -p "$loose"
cp -r "$programs/." "$loose/"
powershell -NoProfile -File "$(cygpath -w "$repo/tools/store/make-images.ps1")" -Out "$(cygpath -w "$loose/Assets")"

description="Worship presentation: songs, Bible verses and media backgrounds."
manifest="$(cat "$repo/tools/store/AppxManifest.xml")"
manifest="${manifest//\{\{NAME\}\}/$(xml "$name")}"
manifest="${manifest//\{\{PUBLISHER\}\}/$(xml "$publisher")}"
manifest="${manifest//\{\{PUBLISHER_DISPLAY_NAME\}\}/$(xml "$display")}"
manifest="${manifest//\{\{VERSION\}\}/$package_version}"
manifest="${manifest//\{\{DESCRIPTION\}\}/$(xml "$description")}"
printf '%s\n' "$manifest" > "$loose/AppxManifest.xml"

rm -f "$msix"
MSYS_NO_PATHCONV=1 "$makeappx" pack /v /h SHA256 /o /d "$(cygpath -w "$loose")" /p "$(cygpath -w "$msix")"
echo "Package $package_version: $msix ($(du -m "$msix" | cut -f1) MB)"

if [ "$name" = "Artosia.Unreserved" ] || [[ "$publisher" == *00000000-0000-0000-0000-000000000000* ]]; then
  echo "THIS PACKAGE IS NOT FOR UPLOAD: tools/store/identity.json STILL HOLDS THE PLACEHOLDER IDENTITY." >&2
  echo "PUT THE NAME, PUBLISHER AND PUBLISHER DISPLAY NAME FROM PARTNER CENTER IN IT AND RUN THIS AGAIN." >&2
fi
