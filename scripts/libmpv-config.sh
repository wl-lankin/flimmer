# The one place that pins Flimmer's macOS libmpv: every source release and its checksum, plus the
# download helpers. Sourced by the build and fetch scripts and the workflows. Changing this file
# changes the CI cache key, so the library is rebuilt. The Windows pin is in libmpv-windows.sh.
#
# Versions and checksums follow the Homebrew formulae of the same name at the time of pinning.

LIBMPV_MACOS_MIN="11.0"

# name|version|url|sha256
LIBMPV_SOURCES="
dav1d|1.5.4|https://code.videolan.org/videolan/dav1d/-/archive/1.5.4/dav1d-1.5.4.tar.bz2|2abfb0c89212e6e4733a54e0ae509ec00a5b845a6360946f918806e14aedb011
freetype|2.15.0|https://downloads.sourceforge.net/project/freetype/freetype2/2.15.0/freetype-2.15.0.tar.xz|93cd47eb719c325107f2d3cf087d06299e944c3b48a40b55537302304dea1dd5
fribidi|1.0.18|https://github.com/fribidi/fribidi/releases/download/v1.0.18/fribidi-1.0.18.tar.xz|1de5e8073d035388b71d0b2d452b1da1f8caeea8a7702997b401fe1b6250727f
harfbuzz|15.1.0|https://github.com/harfbuzz/harfbuzz/releases/download/15.1.0/harfbuzz-15.1.0.tar.xz|0fc7c565443a5f4be7da65ac5ee5bd737d9b158dc908708b7689a0a5796b0c8d
libass|0.17.5|https://github.com/libass/libass/releases/download/0.17.5/libass-0.17.5.tar.xz|2dca25c0e0c837ddf00b52011b3f82cac1e4ddd3ad018227806b0c2288864acc
uchardet|0.0.8|https://www.freedesktop.org/software/uchardet/releases/uchardet-0.0.8.tar.xz|e97a60cfc00a1c147a674b097bb1422abd9fa78a2d9ce3f3fdcc2e78a34ac5f0
lcms2|2.19.1|https://downloads.sourceforge.net/project/lcms/lcms/2.19.1/lcms2-2.19.1.tar.gz|bfc54f7bab59fbc921012014a8032e4cba4abd46db47d46b76416a8c0b2815c8
luajit|c4b1e2f73a2e10fbcc637fcec474f639c38daafe|https://github.com/LuaJIT/LuaJIT/archive/c4b1e2f73a2e10fbcc637fcec474f639c38daafe.tar.gz|8232ea0ebc837a56ef679f8e4c04c2dfc4dec0b4145282337fd618b270b43ddd
fast_float|8.3.3|https://github.com/fastfloat/fast_float/archive/refs/tags/v8.3.3.tar.gz|4dd695f051893c8218faf2ed9871fa4a4990aa6aee2d3190ed7bfa2e83092245
vulkan_headers|1.4.363.0|https://github.com/KhronosGroup/Vulkan-Headers/archive/refs/tags/vulkan-sdk-1.4.363.0.tar.gz|4a078be12bef21cfebc09d878b77a63cff9d68f899254a0b00d0e37ef73e7f7e
libplacebo|7.387.0|https://code.videolan.org/videolan/libplacebo/-/archive/v7.387.0/libplacebo-v7.387.0.tar.bz2|14ceeeb14d1c08fcdb1a782a06129da11f842c6fddbf3a43c1c4061406ad03d2
ffmpeg|9.0.2|https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz|8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e
mpv|0.42.0|https://github.com/mpv-player/mpv/archive/refs/tags/v0.42.0.tar.gz|bcfebbfa2b65a6d88dc97ea18117831072373e38b5785a2afb874ed4df5070dd
"

libmpv_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# Downloads $2 to $1 unless present, then checks it against $3.
libmpv_download() {
  local file="$1" url="$2" sum="$3"
  if [ ! -f "$file" ]; then
    curl -fsSL --retry 3 -o "$file.part" "$url"
    mv "$file.part" "$file"
  fi
  local actual
  actual="$(libmpv_sha256 "$file")"
  if [ "$actual" != "$sum" ]; then
    echo "checksum mismatch for $file: $actual" >&2
    rm -f "$file"
    return 1
  fi
}

# Downloads every macOS source into $1 (checked), named <name>-<version>.<ext>.
libmpv_fetch_sources() {
  local dir="$1" name version url sum
  mkdir -p "$dir"
  while IFS='|' read -r name version url sum; do
    [ -n "$name" ] || continue
    libmpv_download "$dir/$name-$version.${url##*.}" "$url" "$sum"
  done <<<"$LIBMPV_SOURCES"
}
