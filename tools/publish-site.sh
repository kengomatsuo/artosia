#!/usr/bin/env bash
# Builds the homepage and puts it on the gh-pages branch, which GitHub Pages
# serves at artosia.matsuokengo.com.
#   tools/publish-site.sh
# The version and the zip's size are read from the newest release, so the page
# never names a version nobody can download. The branch is replaced each time,
# after the owner's own deploy.sh in kengomatsuo/kengomatsuo.
set -euo pipefail
cd "$(dirname "$0")/.."

repo=kengomatsuo/artosia
read -r tag bytes < <(gh api "repos/$repo/releases/latest" \
  --jq '[.tag_name, (.assets[] | select(.name == "Artosia-windows.zip") | .size)] | join(" ")')
[ -n "${bytes:-}" ] || { echo "the newest release has no Artosia-windows.zip" >&2; exit 1; }

out=target/site
python -I tools/build-site.py --version "${tag#v}" --zip-bytes "$bytes" --out "$out"

remote=$(git remote get-url origin)
name=$(git config user.name)
mail=$(git config user.email)
cd "$out"
git init -q
git checkout -q -b gh-pages
git add -A
git -c user.name="$name" -c user.email="$mail" commit -q -m "Site for ${tag#v}"
git push --force "$remote" gh-pages
cd - > /dev/null
rm -rf "$out/.git"

echo "Site for ${tag#v} is on gh-pages."
