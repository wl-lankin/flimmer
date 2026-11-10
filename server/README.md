# flimmer-together

Kleiner WebSocket-Relay-Server für die Funktion „Gemeinsam schauen“ in Flimmer.

Mehrere Flimmer-Apps treten über einen kurzen Raumcode einem Raum bei. Jede App spielt ihre eigene Kopie der Datei lokal ab. Der Server leitet nur den Wiedergabezustand (Play/Pause, Position, Geschwindigkeit) zwischen den Mitgliedern eines Raums weiter. Es laufen keine Mediendaten über den Server.

## Regeln

- Raumcode: 6 Zeichen aus `ABCDEFGHJKLMNPQRSTUVWXYZ23456789` (ohne 0, O, 1, I), beim Beitritt ohne Beachtung der Groß-/Kleinschreibung.
- Höchstens 8 Mitglieder pro Raum. Ein Raum wird gelöscht, sobald das letzte Mitglied geht oder die Verbindung verliert.
- Anzeigename: getrimmt, höchstens 32 Zeichen, leer wird zu „Gast“.
- Nachrichten sind höchstens 2 KiB groß, größere Frames beenden die Verbindung.
- Höchstens 30 Nachrichten pro Sekunde und Verbindung, darüber hinaus werden Nachrichten stillschweigend verworfen.
- Wer 90 Sekunden nichts sendet, wird getrennt (Clients senden alle 30 Sekunden `ping`).
- Ein Client ist in höchstens einem Raum. `create` und `join` verlassen den alten Raum zuerst.

## Protokoll

JSON-Textframes, das Feld `t` nennt den Typ. Endpunkt: `/together` (alle anderen Pfade antworten mit 404).

| Richtung | Nachricht | Antwort / Wirkung |
| --- | --- | --- |
| Client an Server | `{"t":"create","name":"…"}` | `{"t":"created","room":"K7QX2M","you":"<id>"}` |
| Client an Server | `{"t":"join","room":"K7QX2M","name":"…"}` | `{"t":"joined","room":"K7QX2M","you":"<id>"}` oder `{"t":"error","code":"room-not-found"\|"room-full"\|"bad-request"}` |
| Client an Server | `{"t":"state","paused":bool,"time":<Sekunden>,"speed":<Faktor>,"file":"<Dateiname>","reason":"play"\|"pause"\|"seek"\|"speed"\|"tick"\|"load"}` | Weiterleitung an alle anderen Mitglieder mit zusätzlich `"from":"<id>"`, `"name":"…"`, `"at":<Server-Unixzeit in ms>` |
| Client an Server | `{"t":"leave"}` | verlässt den Raum, die Verbindung bleibt offen |
| Client an Server | `{"t":"ping"}` | `{"t":"pong","at":<Server-Unixzeit in ms>}` (zur Latenzschätzung) |
| Server an Clients | `{"t":"peers","peers":[{"id":"…","name":"…"}]}` | geht bei jeder Änderung der Mitglieder an alle Mitglieder des Raums |
| Server an Client | `{"t":"error","code":"bad-request"}` | bei ungültigem JSON oder unbekanntem `t`, die Verbindung bleibt bestehen |

## Starten

```sh
cargo build --release
FLIMMER_TOGETHER_ADDR=127.0.0.1:24880 ./target/release/flimmer-together
```

`FLIMMER_TOGETHER_ADDR` ist optional, Standard ist `127.0.0.1:24880`. Der Server spricht unverschlüsseltes ws, TLS übernimmt nginx. Log (Verbindungen, Räume anlegen und löschen, ohne Nachrichteninhalte) geht mit Zeitstempel nach stderr. SIGTERM und Strg+C beenden den Server geordnet.

Tests und Prüfungen:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## systemd

`/etc/systemd/system/flimmer-together.service`:

```ini
[Unit]
Description=Flimmer Together relay
After=network.target

[Service]
User=flimmer-together
Group=flimmer-together
Environment=FLIMMER_TOGETHER_ADDR=127.0.0.1:24880
ExecStart=/usr/local/bin/flimmer-together
Restart=on-failure
RestartSec=2
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

Benutzer anlegen und aktivieren:

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin flimmer-together
sudo install -m 755 target/release/flimmer-together /usr/local/bin/
sudo systemctl enable --now flimmer-together
```

## nginx

Im passenden `server { … }`-Block mit TLS:

```nginx
location /together {
    proxy_pass http://127.0.0.1:24880;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_read_timeout 120s;
}
```

Clients verbinden sich dann mit `wss://<host>/together`.
