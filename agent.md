- When writing something intended for human consumption, (comment, commit message, reply to prompt) use as few words as possible. Pick every word meticulously to reduce the volume to a strict minimum. Be down to the point. Less is more.

- Avoid superlatives and praise. Stop telling me I am absolutely right. Give me the cold hard truth.

- Avoid magic numbers and strings by extracting recurring or meaningful values into descriptive constants (const) or enums. Keep self-explanatory, one-off values inline to avoid clutter. If a value comes from a spec (e.g. HTTP 200 OK), use a constant regardless.

- Reduce code indentation. Avoid Arrow Anti-Pattern. Leverage early return and continue.

- Keep function names short. Less than 30 characters.

- Use enums instead of booleans for function parameters.

- Let the reader of the code breathe. Add empty lines between logical blocks of code.

- Add a small, to the point, comment to explain *what* the block does and *why*. Use examples when possible. Propose ASCII drawings to explain complete systems.

- Treat member visibility changes as a breaking design shift. Keep all fields and functions private unless external access is strictly required by the design. Prompt the user for explicit approval before changing any access modifier from private to internal or public.

- Program to levels of abstraction. Lower-level mechanics (e.g., raw hardware I/O, sector parsing, direct socket streams) must be encapsulated in a dedicated driver/abstraction layer. Expose clean, high-level APIs to the rest of the application so calling code works with domain concepts, not raw implementation details.

- Don't touch blocks of code unrelated to the feature you implement. e.g. Don't add comments to a block of code if you did not create it or modify it. As much as possible try to minimize the number of changed lines when implementing a feature.

- Strictly adhere to the layered boundary hierarchy: each layer may only communicate with its immediate neighbor directly below it. Never "punch holes" through layers (e.g., controllers or UI components must never directly call database queries, raw hardware drivers, or low-level network clients; always route through the intermediate service/abstraction layer).

- Always use {}, even on a one-line "if" statement.

When you write a commit message, follow these 7 rules:
Rule 1: Separate the subject line from the body with a single blank line.
Rule 2: Limit the subject line to 50 characters (72 is the absolute hard limit).
Rule 3: Capitalize the first letter of the subject line.
Rule 4: Do not end the subject line with a period.
Rule 5: Use the imperative mood in the subject line (e.g., "Fix bug," "Add feature," 
        not "Fixed" or "Adds"). Test formula: It must complete the sentence: "If applied,
        this commit will [your subject line here]".
Rule 6: Wrap the body text manually at 72 characters to prevent Git formatting issues.
Rule 7: Use the body to explain what and why vs. how. Assume the code explains the how;
        the message must explain the context and reasoning. 

- If the prompt indicates that a bug is being fixed, don't write the fix right away. First write the test. Observe it failing. Then write the fix. And observe the test passing.

## Kontra – Projekt-Wissen

## Zweck
Bass-Trainer für KDE Plasma (Linux zuerst): hört am echten Bass zu (Klinke, Interface;
ab Phase 4 Mikrofon und MIDI), Tab-Lane, Tabulatur und Noten laufen mit, Rückmeldung
zu Ton und Timing. Lernen und einfach Lieder spielen. Bis 1.0 nur Bass.
Konzept und Entscheidungen: `KONZEPT.md`; Anforderungen: `LASTENHEFT.md`;
Personas: `PERSONAS.md`; Journeys: `QA.md`; Plan und QA-Runden: `ROADMAP.md`.

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
- Gitea-Merge per API: `tea api -X POST /repos/shrippen/Kontra/pulls/N/merge -d '{"Do":"merge"}'`.
  „manually-merged“ ist hier nicht erlaubt; „Please try again later“ heißt meist: nichts mehr zu mergen (Inhalt schon in
  `main`). `scripts/gitea/merge-when-green.sh PR SHA` wartet auf die neuesten core- und app-Läufe und merged nur, wenn
  beide grün sind und der Kopf unverändert ist. Die Actions-Liste der API zeigt ohne `limit=50` nur 10 Läufe, und ältere
  abgebrochene Läufe zum selben Commit täuschen: immer nur den neuesten Lauf je Job werten.
- PR-Agent schreibt sein Review einmal und aktualisiert danach denselben Kommentar („Review updated until
  commit <sha>“, neues `updated_at`, altes `created_at`). Ein neues Review erkennt man am Commit im Text, nicht am
  Erstellungsdatum. Kommentare, die nicht mit `/` beginnen, starten keinen Lauf (Status „skipped“).
- Flatpak baut nur GitHub (`.github/workflows/flatpak.yml`): Die Job-Container des Gitea-Runners erlauben keine
  User-Namespaces. Merge-Kriterium auf Gitea sind die Jobs core und app.
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
- Ein Runner für alle Repos, App-Jobs 15–20 min: gestapelte PRs und Pushes auf untere Branches vervielfachen die
  Läufe. Lieber wenige, unabhängige PRs.
- Gitea-Merge „Please try again later“ bei sauberem Merge: die Mergebarkeit ist veraltet;
  `POST pulls/<n>/update?style=merge` stößt die Prüfung neu an.
- Korpus: Synthetische Sätze messen nur, was die Synthese modelliert; Änderungen an der Erkennung immer auch gegen die
  echten IDMT-Aufnahmen messen (`kontra-cli korpus measure`, wie die CI). Techniken genauso: `kontra-cli korpus
  techniques` (12.10.: 91,7 % synthetisch, aber 0 % echte Slaps).
- Neue i18n-Schlüssel nicht in einem fremden Namensraum anlegen (`keys.*` ist die Tastenkürzel-Liste): `keys.next` stand
  doppelt, JSON behielt still den letzten. Ein Test prüft jetzt auf doppelte Schlüssel.
- Prüfkette vor einem Commit mit `&&` oder `set -e`, nicht mit `;`: sonst wird trotz rotem Test committet und gepusht.
- Gestapelte PRs bei einem Runner: Konflikte (i18n-Enden, Importlisten) vorab lösen, indem die untere Branch in die
  obere gemerged wird; bleibt die untere unverändert, ist die obere nach deren Merge ohne neue CI mergebar.
- Tests, die ein Skript schreiben und starten, laufen parallel zu anderen, die Prozesse starten: Ein geforktes Kind hält
  die frische Datei offen, `exec` scheitert mit „Text file busy“ (ETXTBSY, etwa 1 von 40 Läufen). Solche Tests über
  einen gemeinsamen `Mutex` nacheinander laufen lassen (`separator.rs`).
- Seit dem 11.10. abends startet ein `/review`-Kommentar auf Kontra keinen PR-Agent-Lauf mehr (kein Lauf, auch kein
  „skipped“); zweimal fragen und dann nach grüner CI mergen (Regel vom 11.10.).

## Persona rule

- `PERSONAS.md` holds the fixed personas P1–P7. They are used automatically, without being asked:
  - Before designing a flow or screen, name the personas it serves and check it against
    their equipment, knowledge and frustrations.
  - Every PR names the affected personas and the journeys from `QA.md` that were played
    through, with the result (passed, findings with severity).
  - Every requirement added to `LASTENHEFT.md` names at least one persona.
  - Personas whose instrument or input comes later (P5, P6) check that the architecture stays
    open and that Kontra says clearly what is not supported yet.
- Change a persona only on the user's request; never adapt a persona to make a finding go away.

## QA rule

- Exploratory QA follows the user journeys in `QA.md`: play them as the named persona
  (equipment, resolution, scaling, colour filter, input as in `PERSONAS.md`), with a real
  instrument or recordings from `testdata/korpus/`, note friction, illogical behaviour and
  bugs with evidence, and record the round in `ROADMAP.md`.
- Journeys marked ⚙ have an automated test in `tests/journeys/`; keep it in step with the journey.
- A feature that adds or changes a flow adds or changes its journey in `QA.md`.

## Release rule

- Every release gets a detailed release log, written before the tag: a lead (what the release is about), then the changes grouped as New, Improved, Fixed and Note (breaking changes, steps the user must take). Each point says what changed for the user and why, not which code moved.
- Graphics wherever possible: for every visible change a screenshot from the demo world in every language of the landing page (`docs/shots/release-<version>-<name>-<lang>.webp`), for changes inside a diagram (Kante `.flow` or an inline SVG).
- The log goes into the release notes on Gitea and as a new entry at the top of `#changes` on the landing page (`docs/index.html`, Kante `.changelog`); from the fourth entry on, the oldest move into `details.changelog-more`. Bump the version in the footer. A release is done when the landing page shows it.

## GUI rule

- Every GUI of this project is generated from Kante, not inspired by it: landing pages,
  web apps, Qt Quick / Kirigami apps, Plasma widgets, dialogs, e-mail and print layouts.
  Source: https://github.com/shrippen/Kante (checkout `../Kante`).
  Web: link `https://shrippen.github.io/v1/shrippen.css` and `shrippen.js`, or vendor them
  unchanged. Apps: copy `qml/Kante` (and `KantePlasma` for Plasma widgets) unchanged.
- Use Kante's tokens, roles, components, classes, QML components and motion as they are.
  No own colours, fonts, sizes, radii, cuts, shadows, animation timings, no own copy or
  variant of a component that Kante has. Raw values (`#hex`, `px` for controls) are a bug;
  use roles (`--primary`, `--focus`, `--warn`, `KanteStyle.*`).
- A missing element is added to Kante first (CSS or QML, docs, catalogue), then used here.
  Never solve it locally in this project and never wait with a "temporary" copy.
- Exception: Kimai plugins take their GUI from Knust (`kimai/knust/` in Kante) and
  the kit (`kimai/kit/`), the Kante spinoff that adapts Kante to Kimai's look. The same rule applies to Knust: use it
  as it is, and add missing elements to Knust.
- Exception: Kintsugi (`shrippen/kintsugi`) uses Kante Gold (`<html data-kante="gold">`),
  the noble variant defined in Kante itself. The same rule applies: use it as it is, and
  add a missing element or Gold detail to Kante (its Gold block) first. No other project
  uses Kante Gold without a decision recorded here.
- A project without a GUI (library, CLI, scripts) has nothing to do here.
- Here: `qml/Kante` is copied unchanged from Kante (`scripts/sync-kante.sh`); never edit it.
  Kontra's own components (Tab-Lane, notation, fretboard, tuner, level meter, timing
  histogram, transport bar) are added to Kante first, then used here.
- Rule text: https://github.com/shrippen/Kante/blob/main/AGENT-RULE.md

## Repository rule

- This repository lives on Gitea (`git.arianw.de`). GitHub is only a push mirror of it.
- Changes arrive as pull requests: work on a branch, open a PR, merge it on Gitea (the mirror follows).
- Never merge a PR, push to `main` (or any default branch), push tags or publish releases on GitHub. A merge there is overwritten by the next Gitea push.
- Never force-push a branch that someone else's PR depends on.
- PR-Agent (`.gitea/workflows/pr-agent.yml`) reviews every PR before it is merged. Wait for its comment on the PR's latest commit; after further pushes, ask for a new one with a `/review` comment. Fix or answer each finding in the PR, then merge. If the review fails (an error, no result, e.g. a key or cost limit), ask for it once more with `/review`; if that fails too, green CI is enough to merge (owner, 11.10.2026).

## Entscheidungen mit dem Eigentümer

- Offene Fragen sammeln und als Runde im Entscheidungs-Artefakt stellen (je Frage Kontext, Optionen, Empfehlung);
  Antworten liegen dort in der Datenbank-Sammlung „antworten“, ein Dokument je Runde. Umgesetzte Entscheidungen
  kommen ins Lastenheft und in die Roadmap (Abschnitt der jeweiligen Runde).
- Kontra-PRs merged Claude selbst, wenn core und app grün sind und das PR-Agent-Review abgearbeitet ist (gescheitertes
  Review: einmal `/review` wiederholen, scheitert es erneut, genügt grüne CI). Kante-PRs prüft und merged der Eigentümer.
- Jede neue `pub`-Schnittstelle wird in der nächsten Runde zur Freigabe gelistet.
- Der laufende Arbeitsstand steht in `STAND.md`.
