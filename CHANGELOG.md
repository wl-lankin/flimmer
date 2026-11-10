# Changelog

## 0.1.0

Die erste Version von Flimmer.

- Spielt alles, was mpv kann: jedes gängige Format, HDR mit Tonemapping, Untertitel aller Art, Streams und Links.
- Aura: Die Farben am Bildrand leuchten um das Video.
- Pause-Poster mit Titel, Kapitel, Restzeit und der Uhrzeit, zu der der Film endet.
- Studio-Modus (Tab) mit Filmstreifen, Timecode mit Frames, Einzelbildschritt, A-B-Schleife und Technik-Anzeige.
- Weiterschauen: jede Datei startet dort, wo du aufgehört hast, mit Tonspur, Untertitel und Tempo.
- Untertitel erzeugen: whisper.cpp schreibt mit, was gesprochen wird, auf deinem Rechner. Läuft Ollama, wird übersetzt.
- Im Gesprochenen suchen (⌘F / Strg+F).
- Intro überspringen, erkannt am Ton der Folgen im selben Ordner.
- Gemeinsam schauen mit Raumcode.

Installation:

- Windows: `Flimmer-setup.exe` ausführen. Windows SmartScreen kann warnen, weil Flimmer nicht signiert ist: „Weitere Informationen“, dann „Trotzdem ausführen“.
- macOS: `Flimmer.dmg` öffnen und Flimmer in „Programme“ ziehen. Beim ersten Start Rechtsklick auf Flimmer, „Öffnen“, bestätigen. Oder im Terminal: `xattr -dr com.apple.quarantine /Applications/Flimmer.app`
