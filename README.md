<p align="center">
  <img src="docs/logo.png" alt="Flimmer logo" width="128" height="128">
</p>

<h1 align="center">Flimmer</h1>

<p align="center"><strong>Plays everything. Just works.</strong></p>

<p align="center">
  A video player for Windows and macOS, built on mpv.<br>
  Every format, HDR, subtitles of every kind, streams and links.
</p>

> Flimmer is in early development. There is no release yet.

## What makes it different

- **Aura:** the colours at the edge of the picture glow around the video.
- **Pause poster:** pausing turns the frame into a poster with the title, chapter, time left and the time the film ends.
- **Studio mode** (Tab): filmstrip, timecode with frames, frame stepping, A-B loop, technical overlay.
- **Continue watching:** every file resumes where you stopped, with its audio, subtitle and speed.
- **Subtitles from nothing:** whisper.cpp writes subtitles on your computer; a local Ollama translates them.
- **Search what is said** (⌘F / Ctrl+F): jump to the moment a word is spoken, across everything Flimmer transcribed.
- **Skip intro:** Flimmer listens to the episodes in a folder and finds the intro they share. No online database.
- **Watch together:** a room code, everyone plays their own file, pause and position stay in step.

## Keys

| Key | Action |
|---|---|
| Space / K | Play, pause |
| ← → | 5 s back, forward (Shift 30 s, Alt 1 s) |
| ↑ ↓ | Volume |
| , . | One frame back, forward |
| [ ] / Backspace | Slower, faster / normal speed |
| C / A | Next subtitle / audio track |
| L | A-B loop |
| S | Save a still |
| F / Esc | Full screen / leave it |
| Tab | Studio mode |
| I / G | Technical overlay / Aura |
| 0-9 | Jump to 0 %-90 % |
| ⌘V / Ctrl+V | Play a link from the clipboard |
| ⌘F / Ctrl+F | Search what is said |

## Building

Tauri 2 (Rust) with Svelte 5. Video goes through libmpv: on macOS through mpv's render API into an OpenGL layer under the page, on Windows through mpv's own child window.

```sh
brew install mpv          # libmpv for development (macOS)
npm install
npm run tauri dev
```

`brew install mpv` is only for development (it is a GPL build). Releases ship Flimmer's own LGPL libmpv, see [docs/libmpv.md](docs/libmpv.md). On Windows, run `scripts/fetch-libmpv-windows.sh` and put `mpv-build/out/windows/libmpv-2.dll` next to the binary. Releasing: [docs/releasing.md](docs/releasing.md). The watch-together relay lives in [server/](server/).

## License

MIT. libmpv and FFmpeg are used as LGPL libraries.
