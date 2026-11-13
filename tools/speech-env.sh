#!/usr/bin/env bash
# Sets up what whisper.cpp needs to build (CMake and libclang) when it is where it is on the
# owner's PC. Source it:  . tools/speech-env.sh
# It exports PATH and LIBCLANG_PATH, sets ARTOSIA_SPEECH=1, and says in one line what it found
# or what is missing. Without both, ARTOSIA_SPEECH is 0 and nothing is exported.
#
# Override the places with ARTOSIA_CMAKE_BIN and ARTOSIA_LIBCLANG_DIR.

cmake_bin="${ARTOSIA_CMAKE_BIN:-/f/Programs/Microsoft Visual Studio/2022/Community/Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin}"
libclang_dir="${ARTOSIA_LIBCLANG_DIR:-$HOME/.artosia-tools/llvm-23.1.5/bin}"

missing=""
if ! command -v cmake >/dev/null 2>&1 && [ ! -x "$cmake_bin/cmake.exe" ] && [ ! -x "$cmake_bin/cmake" ]; then
    missing="CMake (looked in $cmake_bin and on PATH)"
fi
if [ -z "${LIBCLANG_PATH:-}" ] && [ ! -f "$libclang_dir/libclang.dll" ] && [ ! -f "$libclang_dir/libclang.so" ]; then
    missing="${missing:+$missing, }libclang (looked in $libclang_dir)"
fi

if [ -n "$missing" ]; then
    export ARTOSIA_SPEECH=0
    echo "Speech recognition is off: missing $missing."
else
    if ! command -v cmake >/dev/null 2>&1; then
        export PATH="$cmake_bin:$PATH"
    fi
    if [ -z "${LIBCLANG_PATH:-}" ]; then
        export LIBCLANG_PATH="$libclang_dir"
    fi
    export ARTOSIA_SPEECH=1
    echo "Speech recognition is on: CMake and libclang found (LIBCLANG_PATH=$LIBCLANG_PATH)."
fi
