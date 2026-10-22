# Review guide

Every PR is reviewed before it is merged (rule in `agent.md`): by PR-Agent
(owner, 22.10.2026); a new Claude agent checks the **Wichtig** of earlier
rounds, and reviews when PR-Agent fails. The
reviewer knows nothing of the session that wrote the change: it gets the
PR's intent, its diff and the repository, and this file. The owner does not read code; this review is the second reader.

## What failure looks like here

Kontra is a bass trainer for KDE Plasma: it listens to a real bass and
gives feedback on pitch and timing while tab, notation and the Tab-Lane run
along. A bug that matters is one the player hears, sees or loses data to:

- **Breaks the audio path:** allocation, locks or waiting in the audio
  callback; timing in the wrong clock (file-relative vs. engine clock,
  `Input::started_s()`); latency or xruns that grow.
- **Judges wrongly:** onset or pitch detection that gets worse on the real
  recordings in `testdata/korpus/` (not only the synthetic ones); units
  that do not match their names (ms, cent, Hz).
- **Breaks the layers:** a crate calling past its neighbour below (UI to
  engine or drivers), `unsafe` outside `drivers/` and the cxx-qt bridge,
  `unwrap`/`expect` outside tests, a new `pub` not named in the PR.
- **Looks broken:** own colours or sizes instead of Kante roles, an edit in
  the copied `crates/app/qml/Kante`, information shown only by colour, a
  text in code instead of a translation key (de and en).
- **Brings in what it must not:** a dependency without a reason in the PR
  or under a licence `agent.md` forbids (GPL, MPL, LGPL crates).
- **Lies:** `LASTENHEFT.md`, `ROADMAP.md`, `QA.md`, `KONZEPT.md`, README or
  comments that the change made wrong.

## What to check in particular

- New behaviour has a test; a changed flow changes its journey in `QA.md`
  (and its test in `tests/journeys/` where it is marked ⚙).
- Changes to detection are measured against the real IDMT recordings
  (`kontra-cli korpus measure`), not only the synthetic set.
- Bundled exercises are generated (`kontra-cli exercises`); a generator
  change without new `testdata/exercises` is a bug.
- Bug fixes: does the fix remove the cause, with a test that failed before?
- `agent.md` code rules: braces always, early return, enums not bools.

## What the owner confirms here

"What is published" (rule in `agent.md`) means here: a release (tag,
release notes, the landing page `docs/index.html` with its changelog),
the Flatpak (`packaging/`, `.github/workflows/flatpak.yml`).

## Skip

- `crates/app/qml/Kante/` (copied by `scripts/sync-kante.sh`; check only
  that it is unchanged), `testdata/exercises/` (generated).
- `docs/shots/`, `docs/social-preview.png`, icons, `testdata/korpus/`
  recordings.

## How to report

- Every finding names file and line and the concrete input or state that
  goes wrong. No guess without evidence; if unsure, say so.
- Each finding gets one label: **Wichtig** (a bug from the list above,
  must be fixed or answered before the merge), **Kleinigkeit** (optional,
  at most five) or **Bestand** (the bug was there before this PR; not
  blocking).
- The report starts with the count, e.g. "2 Wichtig, 1 Kleinigkeit" (the
  session puts the heading above it). The count of **Wichtig** holds every
  one still open, also from an earlier round, e.g. "1 Wichtig (offen aus
  Runde 1)". Only when none is open it says plainly: "Nichts zu beheben."
- In a later round: say for each open **Wichtig** of the last round whether
  it is fixed, or whether its answer is accepted, with the reason. PR-Agent
  sees no earlier round: it reports only its own findings with their labels,
  and a new Claude agent closes the open ones (`agent.md`).
- Findings in German, code names as they are.
