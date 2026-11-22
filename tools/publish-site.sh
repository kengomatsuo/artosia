#!/usr/bin/env bash
# Builds the homepage and deploys it to Cloudflare, which serves it at
# artosia.matsuokengo.com.
#   tools/publish-site.sh
# The version and the files' sizes are read from the newest release, so the page
# never names a version nobody can download. The Mac row appears only when the release
# holds the Mac disk image.
#
# The site is a Cloudflare Worker that only serves files (site/deploy/wrangler.jsonc);
# its custom domain made the DNS record and the certificate. wrangler deploys from the Mac.
# Run on the Mac, the script builds and deploys right here. Run on the release PC (tools/publish.sh
# ends with it), it sends the built folder to the Mac over ssh (Tailscale, `Host mac` in the PC's
# ~/.ssh/config) and deploys there. Either way wrangler takes CLOUDFLARE_API_TOKEN from the
# environment or from the Mac's ~/Repositories/artosia/.env (gitignored), else its own sign-in.
# SITE_DRY_RUN=1 on the Mac builds and bundles everything and deploys nothing.
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
# The Mac has only python3; Git Bash on the PC has python.
py="$(command -v python || command -v python3)"
# SITE_LANGS=en,id publishes only those languages, for a language whose pictures are not rendered yet.
"$py" -I tools/build-site.py --version "${tag#v}" --setup-bytes "$setup" --zip-bytes "$zip" \
    --linux-bytes "$linux" --linux-version "${linux_tag#v}" \
    $([ -z "${mac:-}" ] || echo "--mac-bytes $mac --mac-version ${mac_tag#v}") --out "$out" ${SITE_LANGS:+--langs "$SITE_LANGS"}

# The deploy, run on the Mac by bash with one argument: a folder holding public/, wrangler.jsonc and package.json.
deploy='set -eo pipefail
export PATH=$HOME/.bun/bin:/opt/homebrew/bin:$PATH
env_file=$HOME/Repositories/artosia/.env
if [ -z "${CLOUDFLARE_API_TOKEN:-}" ] && [ -f "$env_file" ]; then set -a; . "$env_file"; set +a; fi
cd "$1" && bun install --silent && bunx --no-install wrangler deploy '"${SITE_DRY_RUN:+--dry-run}"' 2>&1 | tail -4'

if [ "$(uname -s)" = Darwin ]; then
    stage="$(mktemp -d)"
    trap 'rm -rf "$stage"' EXIT
    cp -R "$out" "$stage/public"
    cp site/deploy/wrangler.jsonc site/deploy/package.json "$stage/"
    bash -c "$deploy" deploy "$stage"
    [ -z "${SITE_DRY_RUN:-}" ] || { echo "Site for ${tag#v} built and bundled; SITE_DRY_RUN deployed nothing."; exit 0; }
    echo "Site for ${tag#v} is at https://artosia.matsuokengo.com."
    exit 0
fi

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
ssh -o BatchMode=yes mac 'bash -s -- "$HOME/artosia-site"' <<< "$deploy"

echo "Site for ${tag#v} is at https://artosia.matsuokengo.com."
