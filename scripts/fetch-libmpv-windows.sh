#!/usr/bin/env bash
# Puts the pinned LGPL libmpv-2.dll into mpv-build/out/windows. Takes the upstream release,
# falls back to the mirror in this repository, and checks the SHA-256 either way.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
. "$ROOT/scripts/libmpv-config.sh"
DL="$ROOT/mpv-build/dl"
OUT="$ROOT/mpv-build/out/windows"
mkdir -p "$DL" "$OUT"
archive="$DL/$LIBMPV_WINDOWS_FILE"
libmpv_download "$archive" "$LIBMPV_WINDOWS_UPSTREAM" "$LIBMPV_WINDOWS_SHA256" \
  || libmpv_download "$archive" "$LIBMPV_WINDOWS_MIRROR" "$LIBMPV_WINDOWS_SHA256"
seven="$(command -v 7z || command -v 7zz)"
"$seven" x -y -o"$OUT" "$archive" libmpv-2.dll include >/dev/null
echo "libmpv for Windows: $OUT/libmpv-2.dll"
