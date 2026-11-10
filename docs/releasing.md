# Flimmer veröffentlichen

Ein Release entsteht in GitHub Actions aus einem Tag. Veröffentlicht wird nichts automatisch: Der Workflow füllt nur einen Entwurf, den du von Hand veröffentlichst. Der Ablauf ist derselbe wie bei Prism.

## Vor dem Tag

1. `CHANGELOG.md` hat einen Abschnitt `## <version>` für die Version aus `src-tauri/tauri.conf.json`. Er ist der Text des Releases und die Notiz, die der Updater zeigt.
2. Die Version in `package.json` (und `package-lock.json`), `src-tauri/Cargo.toml` und `src-tauri/tauri.conf.json` ist genau der Tag ohne `v`.
3. CI ist auf dem Commit grün.

## Ablauf

`git tag v0.2.0 && git push origin v0.2.0`, oder den Workflow „Release“ von Hand starten und den Tag eintippen.

1. **prepare** prüft Tag, Versionen und Changelog.
2. **libmpv** baut (oder holt aus dem Cache) die libmpv für macOS und lädt die für Windows, beide geprüft (`docs/libmpv.md`).
3. **release** baut erst macOS (universal, DMG und App), dann Windows (NSIS), bündelt libmpv, signiert die Updater-Dateien und hängt alles an einen Entwurf. Ein Tag mit `-` wird Vorabversion.
4. **assets** legt `Flimmer-setup.exe` und `Flimmer.dmg` ohne Versionsnummer dazu, hängt die libmpv-Quellen an und prüft, dass alles da ist:

| Datei | Zweck |
|---|---|
| `Flimmer_<version>_x64-setup.exe` (+ `.sig`) | Windows-Installer, Updater |
| `Flimmer_<version>_universal.dmg` | macOS |
| `Flimmer_universal.app.tar.gz` (+ `.sig`) | macOS-Updater |
| `Flimmer-setup.exe`, `Flimmer.dmg` | feste Download-Links |
| `latest.json` | Updater-Manifest |
| `flimmer-libmpv-sources.tar`, `flimmer-libmpv-build-scripts.tar.gz` | LGPL-Quellen |

## Danach

1. Entwurf auf GitHub öffnen, Text und Dateien prüfen, Installer starten.
2. Entwurf veröffentlichen. Erst dann finden Updater und feste Links die neue Version.
3. Für einen zweiten Versuch erst Entwurf (und Tag) löschen.

## Schlüssel

Der Workflow liest `TAURI_SIGNING_PRIVATE_KEY` und `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` aus den Repo-Secrets. Lokal liegen sie in `~/.tauri/flimmer-updater.key` und `~/.tauri/flimmer-updater.pass`, der öffentliche Schlüssel steht in `src-tauri/tauri.conf.json`.

Die App ist nicht notarisiert. Beim ersten Start auf dem Mac: Rechtsklick auf Flimmer, „Öffnen“.
