# The pinned Windows libmpv. Sourced by fetch-libmpv-windows.sh and the release workflow.

# Windows: the LGPL dev build of zhongfly/mpv-winbuild (mpv built with -Dgpl=false, everything
# linked statically into libmpv-2.dll). Upstream keeps only recent builds; the archive can also
# be mirrored into this repository's "libmpv-windows" release (docs/libmpv.md). Upstream also
# re-uploads builds under the same tag, so the checksum can stop matching without the mirror.
LIBMPV_WINDOWS_FILE="mpv-dev-lgpl-x86_64-20261110-git-d075ec2a28.7z"
LIBMPV_WINDOWS_UPSTREAM="https://github.com/zhongfly/mpv-winbuild/releases/download/2026-11-10-d075ec2a28/$LIBMPV_WINDOWS_FILE"
LIBMPV_WINDOWS_MIRROR="https://github.com/wl-lankin/flimmer/releases/download/libmpv-windows/$LIBMPV_WINDOWS_FILE"
LIBMPV_WINDOWS_SHA256="03e2a563417f0b335be03c044ce092977099c96498e6c7f60a44812b2c46d663"
LIBMPV_WINDOWS_MPV_COMMIT="d075ec2a28"
