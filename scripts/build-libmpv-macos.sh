#!/usr/bin/env bash
# Builds Flimmer's libmpv for macOS: every dependency statically, linked into one
# libmpv.2.dylib per architecture, then joined into a universal library. LGPL only: mpv with
# -Dgpl=false, FFmpeg without --enable-gpl, --enable-nonfree or --enable-version3.
#
#   scripts/build-libmpv-macos.sh [arm64|x86_64|universal]     (default: universal)
#
# Output: mpv-build/out/<arch>/libmpv.2.dylib (+ include/mpv). Needs meson, ninja, cmake, nasm,
# pkg-config and a Python with glad2 and jinja2 (libplacebo generates its OpenGL loader).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
. "$ROOT/scripts/libmpv-config.sh"
WORK="$ROOT/mpv-build"
DL="$WORK/dl"
JOBS="$(sysctl -n hw.ncpu)"
export MACOSX_DEPLOYMENT_TARGET="$LIBMPV_MACOS_MIN"


# Unpacks a fresh copy of source $1 into $SRC/$1 and prints that directory.
unpack() {
  local name="$1" line version url
  line="$(grep "^$name|" <<<"$LIBMPV_SOURCES")"
  version="$(cut -d'|' -f2 <<<"$line")"
  url="$(cut -d'|' -f3 <<<"$line")"
  rm -rf "$SRC/$name"
  mkdir -p "$SRC/$name"
  tar -xf "$DL/$name-$version.${url##*.}" -C "$SRC/$name" --strip-components 1
  echo "$SRC/$name"
}

write_cross_file() {
  local cpu_family="$1"
  cat >"$BUILD/cross.txt" <<EOF
[binaries]
c = ['clang', '-arch', '$ARCH']
cpp = ['clang++', '-arch', '$ARCH']
objc = ['clang', '-arch', '$ARCH']
objcpp = ['clang++', '-arch', '$ARCH']
ar = 'ar'
strip = 'strip'
pkg-config = 'pkg-config'
nasm = 'nasm'

[built-in options]
c_args = ['-mmacosx-version-min=$LIBMPV_MACOS_MIN']
cpp_args = ['-mmacosx-version-min=$LIBMPV_MACOS_MIN']
objc_args = ['-mmacosx-version-min=$LIBMPV_MACOS_MIN']
c_link_args = ['-mmacosx-version-min=$LIBMPV_MACOS_MIN']
cpp_link_args = ['-mmacosx-version-min=$LIBMPV_MACOS_MIN']
objc_link_args = ['-mmacosx-version-min=$LIBMPV_MACOS_MIN']

[properties]
pkg_config_libdir = ['$PREFIX/lib/pkgconfig']

[host_machine]
system = 'darwin'
subsystem = 'macos'
kernel = 'xnu'
cpu_family = '$cpu_family'
cpu = '$cpu_family'
endian = 'little'
EOF
}

meson_build() {
  local dir="$1"
  shift
  (cd "$dir" && meson setup _build --cross-file "$BUILD/cross.txt" --prefix "$PREFIX" --libdir lib \
    --buildtype release --default-library static "$@" && ninja -C _build -j "$JOBS" && ninja -C _build install) >"$BUILD/$(basename "$dir").log" 2>&1 \
    || { tail -40 "$BUILD/$(basename "$dir").log"; exit 1; }
}

build_arch() {
  ARCH="$1"
  local cpu_family host
  if [ "$ARCH" = arm64 ]; then cpu_family=aarch64; host=aarch64-apple-darwin; else cpu_family=x86_64; host=x86_64-apple-darwin; fi
  BUILD="$WORK/build/$ARCH"
  SRC="$BUILD/src"
  PREFIX="$BUILD/prefix"
  # LIBMPV_ONLY_MPV=1 (local use) keeps the dependencies already in $PREFIX and rebuilds mpv.
  local only_mpv="${LIBMPV_ONLY_MPV:-}"
  [ -n "$only_mpv" ] && [ -f "$PREFIX/lib/libavcodec.a" ] || { only_mpv=""; rm -rf "$BUILD"; }
  mkdir -p "$SRC" "$PREFIX/lib/pkgconfig"
  write_cross_file "$cpu_family"
  export PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig"
  unset PKG_CONFIG_PATH
  local flags="-arch $ARCH -mmacosx-version-min=$LIBMPV_MACOS_MIN"
  echo "== $ARCH"

  if [ -z "$only_mpv" ]; then
  echo "-- dav1d"
  meson_build "$(unpack dav1d)" -Denable_tools=false -Denable_tests=false -Denable_examples=false

  echo "-- freetype"
  meson_build "$(unpack freetype)" -Dbrotli=disabled -Dbzip2=disabled -Dharfbuzz=disabled -Dpng=disabled -Dzlib=disabled -Dtests=disabled

  echo "-- fribidi"
  meson_build "$(unpack fribidi)" -Ddocs=false -Dbin=false -Dtests=false

  echo "-- harfbuzz"
  meson_build "$(unpack harfbuzz)" -Dglib=disabled -Dgobject=disabled -Dcairo=disabled -Dicu=disabled -Dchafa=disabled \
    -Dfreetype=enabled -Dcoretext=enabled -Dtests=disabled -Ddocs=disabled -Dutilities=disabled -Dintrospection=disabled -Dbenchmark=disabled

  echo "-- libass"
  meson_build "$(unpack libass)" -Dfontconfig=disabled -Ddirectwrite=disabled -Dcoretext=enabled -Dlibunibreak=disabled \
    -Drequire-system-font-provider=false -Dtest=disabled -Dcompare=disabled -Dprofile=disabled -Dfuzz=disabled -Dcheckasm=disabled

  echo "-- uchardet"
  local dir
  dir="$(unpack uchardet)"
  (cd "$dir" && cmake -S . -B _build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$PREFIX" -DCMAKE_INSTALL_LIBDIR=lib \
    -DCMAKE_OSX_ARCHITECTURES="$ARCH" -DCMAKE_OSX_DEPLOYMENT_TARGET="$LIBMPV_MACOS_MIN" -DBUILD_SHARED_LIBS=OFF -DBUILD_BINARY=OFF \
    -DCMAKE_POLICY_VERSION_MINIMUM=3.5 && ninja -C _build install) >"$BUILD/uchardet.log" 2>&1 || { tail -40 "$BUILD/uchardet.log"; exit 1; }

  echo "-- lcms2"
  meson_build "$(unpack lcms2)" -Dutils=false -Dtests=disabled -Djpeg=disabled -Dtiff=disabled

  echo "-- luajit"
  dir="$(unpack luajit)"
  (cd "$dir" && make -j "$JOBS" amalg BUILDMODE=static HOST_CC="clang" CC="clang" TARGET_FLAGS="$flags" \
    MACOSX_DEPLOYMENT_TARGET="$LIBMPV_MACOS_MIN" PREFIX="$PREFIX" XCFLAGS="-DLUAJIT_ENABLE_LUA52COMPAT" \
    && make install PREFIX="$PREFIX" BUILDMODE=static) >"$BUILD/luajit.log" 2>&1 || { tail -40 "$BUILD/luajit.log"; exit 1; }
  rm -f "$PREFIX"/lib/libluajit*.dylib

  echo "-- libplacebo"
  dir="$(unpack libplacebo)"
  # Header-only submodules the release tarball leaves empty. The Vulkan headers are needed for
  # the stubs even with Vulkan off.
  cp -R "$(unpack fast_float)/include" "$dir/3rdparty/fast_float/"
  cp -R "$(unpack vulkan_headers)/." "$dir/3rdparty/Vulkan-Headers/"
  meson_build "$dir" -Dvulkan=disabled -Dopengl=enabled -Dd3d11=disabled -Dglslang=disabled -Dshaderc=disabled \
    -Dlcms=enabled -Ddovi=enabled -Dlibdovi=disabled -Ddemos=false -Dtests=false -Dbench=false -Dunwind=disabled -Dxxhash=disabled

  echo "-- ffmpeg"
  dir="$(unpack ffmpeg)"
  (cd "$dir" && ./configure --prefix="$PREFIX" --enable-cross-compile --arch="$cpu_family" --target-os=darwin \
    --cc=clang --cxx=clang++ --extra-cflags="$flags -I$PREFIX/include" --extra-ldflags="$flags -L$PREFIX/lib" \
    --pkg-config=pkg-config --pkg-config-flags=--static \
    --disable-programs --disable-doc --disable-debug --enable-static --disable-shared \
    --disable-autodetect --enable-pthreads --enable-zlib --enable-bzlib --enable-iconv \
    --enable-videotoolbox --enable-audiotoolbox --enable-securetransport --enable-libdav1d \
    --disable-avdevice --enable-network && make -j "$JOBS" && make install) >"$BUILD/ffmpeg.log" 2>&1 \
    || { tail -40 "$BUILD/ffmpeg.log"; exit 1; }
  fi

  echo "-- mpv"
  # Flimmer drives mpv through the render API, so mpv's own Cocoa windows (which since 0.42 need
  # Swift and Vulkan) stay out. Hardware decoding then uses videotoolbox-copy.
  # uchardet is C++ and needs the C++ runtime at the final link; iconv is a system library that
  # prefer_static would otherwise skip.
  meson_build "$(unpack mpv)" --default-library shared -Dprefer_static=true -Dc_link_args="-lc++ -liconv -mmacosx-version-min=$LIBMPV_MACOS_MIN" \
    -Dgpl=false -Dlibmpv=true -Dcplayer=false -Dtests=false -Dfuzzers=false \
    -Dlua=luajit -Djavascript=disabled -Duchardet=enabled -Dlcms2=enabled -Dlibarchive=disabled -Dlibbluray=disabled \
    -Ddvdnav=disabled -Dcdda=disabled -Drubberband=disabled -Dzimg=disabled -Djpeg=disabled -Dlibavdevice=disabled \
    -Dsubrandr=disabled -Dvapoursynth=disabled -Dsdl2-audio=disabled -Dsdl2-video=disabled -Dopenal=disabled \
    -Dcocoa=disabled -Dgl=enabled -Dgl-cocoa=disabled -Dplain-gl=enabled -Dvideotoolbox-gl=disabled \
    -Dvulkan=disabled -Dvideotoolbox-pl=disabled -Dswift-build=disabled -Dmacos-cocoa-cb=disabled \
    -Dmacos-media-player=disabled -Dmacos-touchbar=disabled -Dcoreaudio=enabled -Davfoundation=enabled \
    -Dmanpage-build=disabled -Dhtml-build=disabled -Dpdf-build=disabled

  local out="$WORK/out/$ARCH"
  rm -rf "$out"
  mkdir -p "$out/include"
  cp "$PREFIX/lib/libmpv.2.dylib" "$out/"
  install_name_tool -id @rpath/libmpv.2.dylib "$out/libmpv.2.dylib"
  # libmpv needs no rpath of its own; drop any the build left behind.
  otool -l "$out/libmpv.2.dylib" | awk '/LC_RPATH/ { getline; getline; print $2 }' | while read -r rp; do
    install_name_tool -delete_rpath "$rp" "$out/libmpv.2.dylib"
  done
  ln -s libmpv.2.dylib "$out/libmpv.dylib"
  cp -R "$PREFIX/include/mpv" "$out/include/"
  echo "== $ARCH done: $out/libmpv.2.dylib"
}

target="${1:-universal}"
libmpv_fetch_sources "$DL"
case "$target" in
  arm64 | x86_64) build_arch "$target" ;;
  universal)
    build_arch arm64
    build_arch x86_64
    out="$WORK/out/universal"
    rm -rf "$out"
    mkdir -p "$out"
    lipo -create "$WORK/out/arm64/libmpv.2.dylib" "$WORK/out/x86_64/libmpv.2.dylib" -output "$out/libmpv.2.dylib"
    ln -s libmpv.2.dylib "$out/libmpv.dylib"
    cp -R "$WORK/out/arm64/include" "$out/"
    echo "== universal: $out/libmpv.2.dylib"
    ;;
  *) echo "usage: $0 [arm64|x86_64|universal]" >&2; exit 2 ;;
esac
