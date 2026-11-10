# Working rules for this repository

The rules for all shrippen repositories are in `RULES.md` (writing, code, commits, bug
fixes, repository, GUI). It is a synced copy from the rules repository; do not edit it
here. This file holds only what is particular to this repository. A Claude Code session on
the owner's machine has the rules through the global `CLAUDE.md` already and reads
`RULES.md` only for its GUI rule, before work on a user interface.

## Kontra – Projekt-Wissen

## Zweck
Bass-Trainer für KDE Plasma (Linux zuerst): hört am echten Bass zu (Klinke, Interface;
ab Phase 4 Mikrofon und MIDI), Tab-Lane, Tabulatur und Noten laufen mit, Rückmeldung
zu Ton und Timing. Lernen und einfach Lieder spielen. Bis 1.0 nur Bass.
Konzept und Entscheidungen: `planung/KONZEPT.md`; Anforderungen: `planung/LASTENHEFT.md`;
Personas: `planung/PERSONAS.md`; Journeys: `planung/QA.md`; Plan und QA-Runden: `planung/ROADMAP.md`.

Implementierung: Rust (Cargo-Workspace), UI in QML (Kirigami + Kante) über cxx-qt,
Audio über cpal. Erkennung, Time-Stretch, Notensatz und Import sind eigene Implementierungen. Lizenz MIT.

## Schichten (nur zum direkten Nachbarn darunter)
```
ui/         QML-Seiten, Modelle für QML           (Qt 6, Kirigami, Kante)
  ↓
session/    Zeitachse, Modi, Bewertung, Fortschritt
  ↓
engine/     Erkennung (Onset, Tonhöhe), Wiedergabe, Mixer, Instrument-Profile
  ↓
drivers/    rohes Audio-I/O (cpal), MIDI (midir), Dateizugriff
```
- `content/` (Import, Arrangement, Pakete) liegt neben `session/` und wird nur von ihr gerufen.
- UI ruft nie Engine oder Treiber direkt auf.
- Instrument und Eingang sind getrennte Profile/Adapter; ein neues Instrument ändert weder Session noch Bewertung.
- Jede Schicht ist ein eigenes Crate; Abhängigkeiten nur zum Nachbarn darunter (in `Cargo.toml` sichtbar).
- Alles privat; `pub` (auch `pub(crate)`) nur, wo das Design es braucht. Für Kontra generell freigegeben
  (Eigentümer, 11.10.2026): keine Rückfrage, aber jede neue Schnittstelle im PR nennen.

## Konventionen
- Rust (stable), cargo fmt, clippy ohne Warnungen. `make check` vor jedem Commit
  (Tests, Erkennungs-Tests gegen `testdata/korpus/`, Token-Prüfung, cargo-deny).
- Kein `unsafe` außerhalb von `drivers/` und der cxx-qt-Brücke; jedes `unsafe` mit Kommentar, warum es sicher ist.
  Ausnahme: Tests (`tests/`, `#[cfg(test)]`) dürfen `unsafe` für Messhilfen nutzen, etwa einen zählenden Allocator, der Allokationen im Audio-Thread nachweist; auch dort mit Kommentar.
- Kein `unwrap()`/`expect()` außerhalb von Tests; Fehler als eigene Fehlertypen je Crate.
- Audio-Callback: keine Allokation, keine Sperren, kein Warten; Ergebnisse über vorab angelegte
  Ringpuffer (rtrb). Debug-Builds prüfen das mit einem Allokations-Wächter. Jede Änderung im
  Audio-Pfad misst Latenz und Xruns.
- Abhängigkeiten nur unter MIT, Apache-2.0, BSD, ISC, Zlib, Unlicense, CC0 (Schriften: SIL OFL);
  Qt und KDE Frameworks (LGPL) nur dynamisch. Keine GPL, MPL oder LGPL-Crates.
  Ausnahme (Eigentümer, Runde 18, 10.11.2026): das ASIO-SDK von Steinberg unter GPLv3, nur im Windows-Paket;
  dieses Paket wird unter GPLv3 ausgeliefert, der Quelltext bleibt MIT. Umsetzung: ein Cargo-Feature, das allein
  der Windows-Paketbau einschaltet, damit alle anderen Builds ohne GPL bleiben.
  Im Zweifel (Lizenz unklar, kaum gepflegt, nur kleiner Teil gebraucht) selbst bauen.
  Neue Abhängigkeit nur mit Begründung im PR.
- Texte nur über Übersetzungsschlüssel (Deutsch und Englisch); keine Texte im Code.
- Information nie nur über Farbe (Form, Symbol oder Text dazu).
- Zeiten in Millisekunden, Tonhöhen in Cent relativ zum Sollton, Frequenzen in Hz; Einheit im Namen (`offsetMs`, `deviationCent`).

## Gelernte Fehler
- cxx-qt: Nach Änderungen an `crates/app/src/bridge.rs` (neue Property, Invokable oder Signal) kann der vorkompilierte
  QML-Code (qmlcachegen) alte Methodenindizes aufrufen und abstürzen (SIGSEGV in `QMetaObject::method`), wenn sich die
  QML-Dateien selbst nicht geändert haben. Dann `cargo clean -p kontra-app`; `scripts/screenshots.sh` tut das immer.
- Kirigami-Seiten malen den Plattform-Hintergrund; ohne `background: Rectangle { color: KanteStyle.backgroundColor }`
  steht helle Kante-Schrift auf hellem Grund.
- `grabToImage` auf dem Fenster selbst scheitert („no QML engine“); Screenshots nehmen die aktuelle Seite. Was im
  Screenshot erscheinen soll, liegt deshalb in der Seite, nicht in einem Fenster-Dialog.
- rustfmt bricht lange Zeilen um (max_width 100): Skripte, die Quelltext per Textersetzung ändern, finden danach ihre
  Vorlage nicht mehr. Erst lesen, dann ändern.
- QML: Eine eigene Eigenschaft namens `data` (oder `children`, `resources`) auf einem Item überdeckt dessen Kind-Liste;
  die Seite hängt ohne Fehlermeldung. Namen wie `summary` verwenden.
- Die Engine stempelt Noten aus Dateien mit `Pace::Fast` dateirelativ, mit `Pace::RealTime` auf die Engine-Uhr. Eine
  Aufnahme wird über `Input::started_s()` ausgerichtet, nicht über den Zeitpunkt des Aufrufs (Dekodieren dauert).
- Gitea-Merge per API: `gt api -X POST /repos/shrippen/Kontra/pulls/N/merge -f Do=merge`.
  „manually-merged“ ist hier nicht erlaubt; „Please try again later“ heißt meist: nichts mehr zu mergen (Inhalt schon in
  `main`). `scripts/gitea/merge-when-green.sh PR SHA` wartet auf den neuesten Status `github / test` des Commits und
  merged nur, wenn er grün ist und der Kopf unverändert ist.
- PR-Agent schreibt sein Review einmal und aktualisiert danach denselben Kommentar („Review updated until
  commit <sha>“, neues `updated_at`, altes `created_at`). Ein neues Review erkennt man am Commit im Text, nicht am
  Erstellungsdatum. Kommentare, die nicht mit `/` beginnen, starten keinen Lauf (Status „skipped“).
- CI läuft seit 24.10.2026 nur auf GitHub, wie lmix (`.github/workflows/ci.yml`, Jobs core, app, windows und
  windows-app; die Windows-Jobs zählen seit 10.11.2026 mit, Runde 18), und meldet das
  Ergebnis als Status `github / test` an Gitea; Gitea führt nur PR-Agent aus. Flatpak baut ebenfalls GitHub.
- Mehrere Arbeitsstände parallel: je Branch ein `git worktree` und ein eigenes `CARGO_TARGET_DIR`, dazu
  `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`, sonst ist die Platte schnell voll. `cargo clippy` baut kein Binary;
  vor Screenshots `cargo build`.
- i18n-Konflikte beim Mergen: beide Seiten behalten, Kommas glätten, auf doppelte Schlüssel prüfen (JSON lädt sonst
  stillschweigend nur einen).
- Neue Felder in `Library`, `RecentEntry` oder `Run` brechen beim Mergen Struktur-Literale in Tests anderer Branches
  (`RecentEntry` hat kein `Default`). Nach jedem Merge `cargo test --workspace` laufen lassen.
- Mitgelieferte Übungen sind generiert (`kontra-cli exercises <dir>`); die CI vergleicht mit `testdata/exercises`.
  Nach Änderungen am Generator neu erzeugen und einchecken (`README.md` dort ist handgeschrieben).
- `KanteCallout.Kind` kennt `Ok`, nicht `Success`. Kante-Bauteile ersetzen QQC2 (`KanteSwitch`, `KanteComboBox`),
  sobald Kante sie hat.
- Screenshots von Kante-Bauteilen außerhalb der App: Hintergrund-`Rectangle` mit `Kirigami.Theme`-Farben und
  `Kirigami.Theme.inherit: false` setzen, sonst schwarzer Grund oder falsches Farbschema.
- Der CI-Runner hat englische Systemsprache: Tests mit übersetzten Texten setzen die Sprache selbst; vor dem Push
  `LANG=C cargo test`.
- PR-Agent: Ein normaler PR-Kommentar (ohne `/`) brach ein laufendes Review ab (behoben in #25). Das Review steht
  im ersten Kommentar und wird dort aktualisiert („updated until commit“).
- Ein Gitea-Runner für alle Repos (PR-Agent): gestapelte PRs vervielfachen die Review-Läufe. Lieber wenige,
  unabhängige PRs.
- Gitea-Merge „Please try again later“ bei sauberem Merge: die Mergebarkeit ist veraltet;
  `POST pulls/<n>/update?style=merge` stößt die Prüfung neu an.
- Korpus: Synthetische Sätze messen nur, was die Synthese modelliert; Änderungen an der Erkennung immer auch gegen die
  echten IDMT-Aufnahmen messen (`kontra-cli korpus measure`, wie die CI). Techniken genauso: `kontra-cli korpus
  techniques` (12.10.: 91,7 % synthetisch, aber 0 % echte Slaps).
- Neue i18n-Schlüssel nicht in einem fremden Namensraum anlegen (`keys.*` ist die Tastenkürzel-Liste): `keys.next` stand
  doppelt, JSON behielt still den letzten. Ein Test prüft jetzt auf doppelte Schlüssel.
- Prüfkette vor einem Commit mit `&&` oder `set -e`, nicht mit `;`: sonst wird trotz rotem Test committet und gepusht.
  `set -e` wirkt nicht, wenn die ganze Kette selbst Teil einer `&&`/`||`-Liste ist (so betten manche Werkzeuge
  Befehle ein): am 14.10. ging ein Push trotz Build-Fehler durch. Sicher ist nur `a && b && git push`.
- Kontra im Testlauf nur über `scripts/qa-session.sh --input-file <Datei> [--a11y] [--run <Befehl>] -- <Argumente>`
  starten (eigener Xvfb, xcb ohne Wayland, eigenes `XDG_RUNTIME_DIR` und Datenordner mit `kontra:null`, eigener
  D-Bus nur mit `--a11y`, räumt auf). Vorfälle ohne diese Trennung: 13.10. öffnete Kontra das echte Interface, und
  WirePlumber legte den Eingang des Scarlett Solo nicht wieder an (Abhilfe: neu einstecken oder
  `systemctl --user restart wireplumber`; nie PipeWire-Knoten anderer zerstören oder Profile umstellen); 28.10. ein
  Fenster auf dem echten Desktop (Qt nahm `WAYLAND_DISPLAY`) und ein Portal am echten Wayland (Eigentümer). Tests in
  Rust genauso: ohne `input_file` öffnet `Core::play` das echte Mikrofon (#162).
- Neue Abhängigkeiten: `cargo deny check licenses bans sources` (`~/.cargo/bin/cargo-deny`) vor dem Push; transitive
  Lizenzen fallen sonst erst in der CI auf (14.10.: `webpki-root-certs` über ureqs `platform-verifier`).
- Gestapelte PRs: Konflikte (i18n-Enden, Importlisten) vorab lösen, indem die untere Branch in die
  obere gemerged wird; bleibt die untere unverändert, ist die obere nach deren Merge ohne neue CI mergebar.
- Tests, die ein Skript schreiben und starten, laufen parallel zu anderen, die Prozesse starten: Ein geforktes Kind hält
  die frische Datei offen, `exec` scheitert mit „Text file busy“ (ETXTBSY, etwa 1 von 40 Läufen). Solche Tests über
  einen gemeinsamen `Mutex` nacheinander laufen lassen (`separator.rs`).

## Persona rule

- `planung/PERSONAS.md` holds the fixed personas P1–P8. They are used automatically, without being asked:
  - Before designing a flow or screen, name the personas it serves and check it against
    their equipment, knowledge and frustrations.
  - Every PR names the affected personas and the journeys from `planung/QA.md` that were played
    through, with the result (passed, findings with severity).
  - Every requirement added to `planung/LASTENHEFT.md` names at least one persona.
  - Personas whose instrument or input comes later (P5, P6) check that the architecture stays
    open and that Kontra says clearly what is not supported yet.
- Change a persona only on the user's request; never adapt a persona to make a finding go away.

## QA rule

- Exploratory QA follows the user journeys in `planung/QA.md`: play them as the named persona
  (equipment, resolution, scaling, colour filter, input as in `planung/PERSONAS.md`), with a real
  instrument or recordings from `testdata/korpus/`, note friction, illogical behaviour and
  bugs with evidence, and record the round in `planung/ROADMAP.md`.
- Journeys marked ⚙ have an automated test in `tests/journeys/`; keep it in step with the journey.
- A feature that adds or changes a flow adds or changes its journey in `planung/QA.md`.

## Release

Release steps: `RELEASE.md`. Before a release, load the skill `release-changelog`. A release
is published, so the owner confirms it in words.

## GUI (this project)

- Here: `qml/Kante` is copied unchanged from Kante (`scripts/sync-kante.sh`); never edit it.
  Kontra's own components (Tab-Lane, notation, fretboard, tuner, level meter, timing
  histogram, transport bar) are added to Kante first, then used here.

## Repository (this project)

- The merge waits for: CI is green on the PR's latest commit: the newest status `github / test` there is success (`.github/workflows/ci.yml` on the GitHub mirror, jobs `core`, `app`, `windows` and `windows-app`, every pushed commit; `scripts/gitea/merge-when-green.sh`). No status there yet means wait, not merge. PR-Agent's status and the Flatpak build do not count here.
- The owner also confirms: a PR that changes `packaging/`.
- Everything else: `RULES.md` (Repository) and the skill `pr-review`.

## Entscheidungen mit dem Eigentümer

- Offene Fragen sammeln und als Runde im Entscheidungs-Artefakt stellen (je Frage Kontext, Optionen, Empfehlung);
  Antworten liegen dort in der Datenbank-Sammlung „antworten“, ein Dokument je Runde. Umgesetzte Entscheidungen
  kommen ins Lastenheft und in die Roadmap (Abschnitt der jeweiligen Runde).
- Kontra-PRs merged Claude selbst, wenn `github / test` grün ist und das Review kein **Wichtig** offen lässt (siehe
  Repository (this project) und den Skill `pr-review`). Kante-PRs ebenso: Claude merged sie selbst nach Review, wenn die CI grün ist und kein **Wichtig**
  offen bleibt (Kantes `agent.md`; Eigentümer, Runde 15, 28.10.2026).
- Jede neue `pub`-Schnittstelle wird in der nächsten Runde zur Freigabe gelistet.
- Der laufende Arbeitsstand steht in `planung/STAND.md`.
