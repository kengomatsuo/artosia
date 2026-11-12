#!/usr/bin/env bash
# Builds the homepage and deploys it to Cloudflare, which serves it at
# artosia.matsuokengo.com.
#   tools/publish-site.sh
# The version and the two files' sizes are read from the newest release, so the page
# never names a version nobody can download.
#
# The site is a Cloudflare Worker that only serves files (site/deploy/wrangler.jsonc);
# its custom domain made the DNS record and the certificate. wrangler is signed in on
# the Mac, so the built folder goes there over WSL's SSH and is deployed from there.
set -euo pipefail
cd "$(dirname "$0")/.."

repo=kengomatsuo/artosia
read -r tag setup zip < <(gh api "repos/$repo/releases/latest" --jq '[.tag_name,
  (.assets | map(select(.name == "Artosia-setup.exe")) | .[0].size // ""),
  (.assets | map(select(.name == "Artosia-windows.zip")) | .[0].size // "")] | join(" ")')
[ -n "${setup:-}" ] && [ -n "${zip:-}" ] || { echo "the newest release lacks the setup or the zip" >&2; exit 1; }

out=target/site
# SITE_LANGS=en,id publishes only those languages, for a language whose pictures are not rendered yet.
python -I tools/build-site.py --version "${tag#v}" --setup-bytes "$setup" --zip-bytes "$zip" --out "$out" ${SITE_LANGS:+--langs "$SITE_LANGS"}

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
