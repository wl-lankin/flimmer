# Code-Signierung (Windows)

`flimmer.exe`, `libmpv-2.dll` und der Installer werden mit [Azure Artifact Signing](https://learn.microsoft.com/azure/artifact-signing/) signiert, Herausgeber „Wolfgang Linz“. Aufbau wie bei Orbo:

- Azure: Konto `wolfganglinz-signing` (Endpunkt `https://neu.codesigning.azure.net`), Profil `wolfganglinz-public`. Die App-Registrierung `github-code-signing` hat kein Passwort, die Verbundanmeldeinformation `flimmer-release` vertraut GitHub-OIDC-Tokens für `repo:wl-lankin@27097016/flimmer@1452606272:environment:release`.
- GitHub: Umgebung `release`, Variablen `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID` (IDs, keine Geheimnisse).
- `.github/actions/windows-signing` lädt den Signier-Client (Version und SHA-256 fest) und erzeugt eine Tauri-Konfiguration mit `bundle.windows.signCommand`. Der Release-Workflow signiert `libmpv-2.dll` vor dem Build (Tauri signiert Ressourcen nicht) und prüft danach alle drei Signaturen.
