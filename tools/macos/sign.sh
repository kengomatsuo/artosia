#!/usr/bin/env bash
# Signs the Mac app with the Developer ID of this Mac's keychain, has Apple notarize it, and
# makes the disk image, as Cutling's `deploy.sh dist` does for an Xcode app:
#
#   tools/macos/sign.sh <Artosia.app> <Artosia-macos-aarch64.dmg>
#
# It must run where the login keychain is open (a Terminal of the person signed in; over SSH
# the keychain is locked, and tools/macos/in-login-session.sh carries it across).
#
#   ARTOSIA_MAC_IDENTITY     the signing identity, "Developer ID Application" by default
#   ARTOSIA_NOTARY_PROFILE   the keychain profile `xcrun notarytool store-credentials` made;
#                            "cutling-notary" by default, the one this Mac already holds
#
# Apple's rules for a notarized program ("Resolving common notarization issues"): a Developer
# ID Application certificate, the hardened runtime, a secure timestamp, and every program file
# inside signed. The notary service takes a ZIP or a disk image and makes a ticket for each
# file inside; a ticket can be stapled to an app and to a disk image, never to a ZIP
# ("Customizing the notarization workflow"). So the app is notarized and stapled first, then
# the image that holds the stapled app, and a Mac with no network still opens both.
set -euo pipefail

app="${1:?Give the app}"; image="${2:?Give the disk image to write}"
here="$(cd "$(dirname "$0")" && pwd)"
identity="${ARTOSIA_MAC_IDENTITY:-Developer ID Application}"
profile="${ARTOSIA_NOTARY_PROFILE:-cutling-notary}"
contents="$app/Contents"
work="$(dirname "$image")"

xcrun notarytool history --keychain-profile "$profile" > /dev/null 2>&1 || {
    echo "The notary profile '$profile' cannot be read here (is the keychain open?)."; exit 1; }

# Inside out: libraries and plugins, the helper, the two programs beside the operator, then
# the app, whose signature seals all of them.
sign=(codesign --force --options runtime --timestamp --sign "$identity")
{
    find "$contents/Resources/gstreamer" -type f -name '*.dylib' -print0 | xargs -0 -n 20 "${sign[@]}"
    "${sign[@]}" "$contents/Resources/gstreamer/libexec/gstreamer-1.0/gst-plugin-scanner"
    "${sign[@]}" --entitlements "$here/entitlements/output.plist" "$contents/MacOS/artosia-output"
    "${sign[@]}" --entitlements "$here/entitlements/stream.plist" "$contents/MacOS/artosia-stream"
    "${sign[@]}" --entitlements "$here/entitlements/operator.plist" "$app"
} 2>&1 | { grep -v "replacing existing signature" || true; }
codesign --verify --deep --strict "$app"
echo "Signed: $(codesign -dvv "$app" 2>&1 | grep -c '^Authority=Developer ID Application') Developer ID authority, hardened runtime $(codesign -dvv "$app" 2>&1 | grep -c 'flags=.*runtime')"

zip="$work/Artosia-notarize.zip"
/usr/bin/ditto -c -k --keepParent "$app" "$zip"
xcrun notarytool submit "$zip" --keychain-profile "$profile" --wait --timeout 45m | grep -E "id:|status:" | tail -2
rm -f "$zip"
xcrun stapler staple -q "$app"
xcrun stapler validate -q "$app"

stage="$work/dmg"
rm -rf "$stage"; mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
install -m 644 "$here/README.txt" "$stage/Read me first.txt"
hdiutil create -volname "Artosia" -srcfolder "$stage" -ov -format UDZO -quiet "$image"
rm -rf "$stage"
codesign --force --timestamp --sign "$identity" "$image"
xcrun notarytool submit "$image" --keychain-profile "$profile" --wait --timeout 45m | grep -E "id:|status:" | tail -2
xcrun stapler staple -q "$image"
xcrun stapler validate -q "$image"
echo "Gatekeeper on the app: $(spctl --assess --type execute -vv "$app" 2>&1 | tr '\n' ' ' | sed -E 's/origin=.*//')"
echo "Notarized: $image"
