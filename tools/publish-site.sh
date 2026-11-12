#!/usr/bin/env bash
# Builds the homepage and deploys it to Cloudflare, which serves it at
# artosia.matsuokengo.com.
#   tools/publish-site.sh
# The version and the files' sizes are read from the newest release, so the page
# never names a version nobody can download. The Mac row appears only when the release
# holds the Mac disk image.
#
# The site is a Cloudflare Worker that only serves files (site/deploy/wrangler.jsonc);
# its custom domain made the DNS record and the certificate. wrangler is signed in on
# the Mac, so the built folder goes there over WSL's SSH and is deployed from there.
set -euo pipefail
cd "$(dirname "$0")/.."

repo=kengomatsuo/artosia
read -r tag setup zip linux mac < <(gh api "repos/$repo/releases/latest" --jq '[.tag_name,
  (.assets | map(select(.name == "Artosia-setup.exe")) | .[0].size // "-"),
  (.assets | map(select(.name == "Artosia-windows.zip")) | .[0].size // "-"),
  (.assets | map(select(.name == "Artosia-linux-x86_64.tar.gz")) | .[0].size // "-"),
  (.assets | map(select(.name == "Artosia-macos-aarch64.dmg")) | .[0].size // "-")] | join(" ")')
# A file the release lacks reads as "-", so the fields after it keep their places.
[ "${setup:--}" != - ] && [ "${zip:--}" != - ] && [ "${linux:--}" != - ] || { echo "the newest release lacks the setup, the zip or the Linux package" >&2; exit 1; }

out=target/site
# SITE_LANGS=en,id publishes only those languages, for a language whose pictures are not rendered yet.
python -I tools/build-site.py --version "${tag#v}" --setup-bytes "$setup" --zip-bytes "$zip" --linux-bytes "$linux" $([ "${mac:--}" = - ] || echo "--mac-bytes $mac") --out "$out" ${SITE_LANGS:+--langs "$SITE_LANGS"}

here=$(wsl.exe -d Ubuntu -e wslpath -a "$(cygpath -m "$PWD")" | tr -d '\r\0')
wsl.exe -d Ubuntu -e bash -lc "
  set -e
  ssh -o BatchMode=yes -o ConnectTimeout=10 mac 'mkdir -p ~/artosia-site/public' \
    || { echo 'The Mac cannot be reached, so the site was built and not deployed.' >&2; exit 1; }
  rsync -a --delete '$here/$out/' mac:artosia-site/public/
  scp -q '$here/site/deploy/wrangler.jsonc' '$here/site/deploy/package.json' mac:artosia-site/
  ssh -o BatchMode=yes mac 'export PATH=\$HOME/.bun/bin:/opt/homebrew/bin:\$PATH
    cd ~/artosia-site && bun install --silent && bunx --no-install wrangler deploy' | tail -4
" | tr -d '\0'

echo "Site for ${tag#v} is at https://artosia.matsuokengo.com."
