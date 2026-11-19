#!/usr/bin/env bash
# Prints the one folder every release build and package of Artosia goes to: the programs, the
# update feed, the setup, the zip, the Store package and the Mac and Linux editions, each under
# <version>/. It is the user's temp folder, never the repo, so no built copy of the app sits in a
# checkout: on the Mac Spotlight does not index it and dirhelper deletes what sits there untouched
# for three days. On Windows (Git Bash) and Linux it is $TMPDIR or /tmp.
if [ "$(uname -s)" = Darwin ]; then t="$(getconf DARWIN_USER_TEMP_DIR)"; else t="${TMPDIR:-/tmp}"; fi
printf '%s\n' "${t%/}/builds/artosia"
