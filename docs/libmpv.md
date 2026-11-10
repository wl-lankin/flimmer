# libmpv in Flimmer

Flimmer spielt alles über libmpv. Ausgeliefert wird nur eine LGPL-Fassung: mpv mit `-Dgpl=false`, FFmpeg ohne `--enable-gpl`, `--enable-nonfree` und `--enable-version3`. Flimmer selbst steht unter MIT und bindet libmpv dynamisch ein, die Bibliothek lässt sich also austauschen.

## macOS

`scripts/build-libmpv-macos.sh` baut alles aus Quellen, die in `scripts/libmpv-config.sh` mit Version und SHA-256 festgelegt sind (übernommen aus den Homebrew-Formeln):

| Teil | Lizenz | Wofür |
|---|---|---|
| mpv 0.42.0 | LGPL 2.1+ | Wiedergabe, Render-API |
| FFmpeg 9.0.2 | LGPL 2.1+ | Container, Decoder, Netzwerk (TLS über Secure Transport) |
| libplacebo 7.387.0 | LGPL 2.1+ | Bildverarbeitung, HDR-Tonemapping |
| dav1d 1.5.4 | BSD-2 | AV1 |
| libass 0.17.5 | ISC | ASS/SSA-Untertitel |
| freetype 2.15.0 | FTL | Schriften |
| fribidi 1.0.18 | LGPL 2.1+ | Schreibrichtung |
| harfbuzz 15.1.0 | MIT | Textsatz |
| uchardet 0.0.8 | MPL 1.1 | Zeichensatz von Untertiteln erkennen |
| lcms2 2.19.1 | MIT | Farbmanagement |
| LuaJIT (c4b1e2f) | MIT | mpv-Skripte, z. B. ytdl_hook |
| fast_float 8.3.3 | Apache 2.0/MIT | nur Header, für libplacebo |
| Vulkan-Headers 1.4.363.0 | Apache 2.0 | nur Header, für libplacebo |

Alle Abhängigkeiten werden statisch gebaut und in eine einzige `libmpv.2.dylib` gelinkt, je einmal für arm64 und x86_64, danach mit `lipo` vereint. Die Bibliothek nennt sich `@rpath/libmpv.2.dylib` und liegt im App-Bundle unter `Contents/Frameworks`.

`scripts/check-libmpv-macos.sh` prüft: beide Architekturen, der Name, nur System-Bibliotheken als Abhängigkeiten, kein GPL-Schalter.

Lokal bauen (dauert je Architektur einige Minuten):

```sh
brew install meson ninja cmake nasm pkgconf
python3 -m venv mpv-build/venv && mpv-build/venv/bin/pip install meson glad2 jinja2
PATH="$PWD/mpv-build/venv/bin:$PATH" scripts/build-libmpv-macos.sh universal
MPV_LIB_DIR="$PWD/mpv-build/out/universal" npm run tauri dev
```

Ohne `MPV_LIB_DIR` linkt die Entwicklung gegen die libmpv von Homebrew (`brew install mpv`). Die ist GPL und nur zum Entwickeln gedacht.

## Windows

Windows nutzt den LGPL-Build von [zhongfly/mpv-winbuild](https://github.com/zhongfly/mpv-winbuild) (`mpv-dev-lgpl-x86_64`, mit `-Dgpl=false`, alles statisch in `libmpv-2.dll`). `scripts/fetch-libmpv-windows.sh` lädt die festgelegte Fassung und prüft die SHA-256. Rust bindet die DLL per `raw-dylib`, eine Import-Bibliothek ist nicht nötig. Der Installer legt `libmpv-2.dll` neben `flimmer.exe`.

Upstream hält nur die letzten Builds vor. Damit ein Release auch später noch baut, kann das Archiv in diesem Repo gespiegelt werden, das Skript nutzt den Spiegel als Ausweichquelle:

```sh
gh release create libmpv-windows -R wl-lankin/flimmer --prerelease \
  --title "libmpv für Windows (Build-Abhängigkeit)" --notes "Gespiegelter LGPL-Build, siehe docs/libmpv.md" \
  mpv-build/dl/mpv-dev-lgpl-x86_64-20261110-git-d075ec2a28.7z
```

## Quellen im Release

Jedes Release enthält `flimmer-libmpv-sources.tar` (alle Quellarchive oben und mpv im Stand des Windows-Builds) und `flimmer-libmpv-build-scripts.tar.gz` (diese Skripte und diese Datei).
