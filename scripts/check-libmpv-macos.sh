#!/usr/bin/env bash
# Rejects a macOS libmpv that is not what Flimmer ships: both architectures, @rpath install
# name, only system libraries and frameworks as dependencies, and LGPL only.
#
#   scripts/check-libmpv-macos.sh mpv-build/out/universal/libmpv.2.dylib
set -euo pipefail

lib="${1:?usage: check-libmpv-macos.sh <libmpv.2.dylib>}"
failed=0
fail() { echo "FAIL: $*" >&2; failed=1; }

archs="$(lipo -archs "$lib")"
for a in arm64 x86_64; do
  [[ " $archs " == *" $a "* ]] || fail "architecture $a missing (have: $archs)"
done

[ "$(otool -D "$lib" | tail -1)" = "@rpath/libmpv.2.dylib" ] || fail "install name is not @rpath/libmpv.2.dylib"

for a in arm64 x86_64; do
  # First line names the file, the second is the library's own install name.
  while read -r dep _; do
    case "$dep" in
      /usr/lib/* | /System/Library/*) ;;
      *) fail "$a links a non-system library: $dep" ;;
    esac
  done < <(otool -arch "$a" -L "$lib" | tail -n +3)
done

text="$(strings -a "$lib")"
grep -q -- "-Dgpl=false" <<<"$text" || fail "mpv was not configured with -Dgpl=false"
for flag in --enable-gpl --enable-nonfree --enable-version3; do
  if grep -q -- "$flag" <<<"$text"; then fail "FFmpeg was configured with $flag"; fi
done

if [ "$failed" = 1 ]; then
  echo "libmpv check failed" >&2
  exit 1
fi
echo "libmpv ok: $archs, $(du -h "$lib" | cut -f1)"
