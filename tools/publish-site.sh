#!/usr/bin/env bash
# Builds the homepage and deploys it to Cloudflare, which serves it at
# artosia.matsuokengo.com.
#   tools/publish-site.sh
# The version and the files' sizes are read from the newest release, so the page
# never names a version nobody can download. The Mac row appears only when the release
# holds the Mac disk image.
#
# The site is a Cloudflare Worker that only serves files (site/deploy/wrangler.jsonc);
# its custom domain made the DNS record and the certificate. wrangler is signed in on the
# Mac, so the built folder goes there over ssh from Windows itself (Tailscale, `Host mac` in
# ~/.ssh/config) and is deployed from there. WSL is off limits since 2026-11-14.
set -euo pipefail
cd "$(dirname "$0")/.."

repo=kengomatsuo/artosia
read -r tag setup zip < <(gh api "repos/$repo/releases/latest" --jq '[.tag_name,
  (.assets | map(select(.name == "Artosia-setup.exe")) | .[0].size // "-"),
  (.assets | map(select(.name == "Artosia-windows.zip")) | .[0].size // "-")] | join(" ")')
# A file the release lacks reads as "-", so the fields after it keep their places.
[ "${setup:--}" != - ] && [ "${zip:--}" != - ] || { echo "the newest release lacks the setup or the zip" >&2; exit 1; }
# The Linux and Mac files come from the newest release that carries each: a release goes out
# without one when that system's machine could not be reached, and its row must not vanish.
newest_with() {
    gh api "repos/$repo/releases?per_page=30" --jq "[.[] | select((.draft or .prerelease) | not) | select(any(.assets[]; .name == \"$1\"))][0] | [.tag_name, (.assets[] | select(.name == \"$1\") | .size)] | join(\" \")"
}
read -r linux_tag linux < <(newest_with Artosia-linux-x86_64.tar.gz)
read -r mac_tag mac < <(newest_with Artosia-macos-aarch64.dmg) || true
[ -n "${linux:-}" ] || { echo "no release carries the Linux package" >&2; exit 1; }

out=target/site
# SITE_LANGS=en,id publishes only those languages, for a language whose pictures are not rendered yet.
python -I tools/build-site.py --version "${tag#v}" --setup-bytes "$setup" --zip-bytes "$zip" \
    --linux-bytes "$linux" --linux-version "${linux_tag#v}" \
    $([ -z "${mac:-}" ] || echo "--mac-bytes $mac --mac-version ${mac_tag#v}") --out "$out" ${SITE_LANGS:+--langs "$SITE_LANGS"}

ssh -o BatchMode=yes mac 'mkdir -p ~/artosia-site' \
    || { echo "The Mac cannot be reached over ssh, so the site was built and not deployed." >&2; exit 1; }
# The folder goes over as one archive cut into 4 MB parts, each copied on its own, checked by its
# SHA-256 and tried up to five times: Tailscale dropped the one long stream this used to be (0.1.34),
# as it dropped the Mac edition's copy home (tools/publish.sh `copy_from_mac`).
parts="$(mktemp -d)"
tar -C "$out" -cf - . | split -b 4m - "$parts/part."
(cd "$parts" && sha256sum part.* > sums)
ssh -o BatchMode=yes mac 'rm -rf ~/artosia-site/incoming && mkdir -p ~/artosia-site/incoming'
while read -r sum name <&3; do
    name="${name#\*}"
    for try in 1 2 3 4 5; do
        if scp -q -o BatchMode=yes -o ServerAliveInterval=15 "$parts/$name" "mac:artosia-site/incoming/$name"             && [ "$(ssh -n -o BatchMode=yes mac "shasum -a 256 ~/artosia-site/incoming/$name" | cut -d' ' -f1)" = "$sum" ]; then
            continue 2
        fi
        echo "Part $name of the site reached the Mac damaged or not at all (try $try of 5)."
        sleep 5
    done
    echo "The site could not be copied to the Mac." >&2
    exit 1
done 3< "$parts/sums"
rm -rf "$parts"
ssh -n -o BatchMode=yes mac 'set -e; cd ~/artosia-site && rm -rf public.new && mkdir -p public.new && cat incoming/part.* | tar -xf - -C public.new && rm -rf public incoming && mv public.new public' \
    || { echo "The site's parts did not unpack on the Mac, so nothing was deployed." >&2; exit 1; }
scp -q site/deploy/wrangler.jsonc site/deploy/package.json mac:artosia-site/
ssh -o BatchMode=yes mac 'export PATH=$HOME/.bun/bin:/opt/homebrew/bin:$PATH
  set -o pipefail
  cd ~/artosia-site && bun install --silent && bunx --no-install wrangler deploy 2>&1 | tail -4'

echo "Site for ${tag#v} is at https://artosia.matsuokengo.com."
