# Lektionen (LH-LRN-07 bis -14)

Kurse, Lektionen und Glossar, die Kontra mitbringt. Plan und Begründung: [`planung/LERNEN.md`](../planung/LERNEN.md).
Kontra bettet diesen Ordner beim Bauen ein (`crates/content/build.rs`); ein Test prüft ihn (`cargo test -p
kontra-content --test lessons`). Eigene Lektionen und nachgeladene Kurse nutzen dasselbe Format.

```text
lessons/
├── courses/<kurs>.json      Kapitel, Reihenfolge, Abschlussübung
├── units/<id>/de.md, en.md  eine Lektion je Ordner, eine Datei je Sprache
└── glossary/de.md, en.md    „Was ist …?“, ein Abschnitt je Begriff
```

## Für wen (`for`)

Eine Lektion, ein Begriff oder eine Variante gilt für eine Stufe; Kontra leitet die Stufen aus dem Instrument-Profil
ab, vom Besonderen zum Allgemeinen:

| Profil | Stufen |
|---|---|
| E-Bass mit Bünden | `e-bass`, `fretted`, `strings`, `bass`, `any` |
| E-Bass bundlos | `e-bass`, `fretless`, `strings`, `bass`, `any` |
| Kontrabass | `double-bass`, `strings`, `bass`, `any` |
| Bass am Keyboard | `keys`, `bass`, `any` |

Eine Lektion steht auf der allgemeinsten Stufe, auf der sie stimmt; Besonderes steht in `::: variant`. Ein Kurs gilt
für eine Stufe (`bass`: alle Bass-Profile). Was eine Lektion braucht (`needs`) und verlinkt (`[[id]]`), gilt
mindestens so weit wie die Lektion oder ihre Variante; die Prüfung meldet, wo nicht.

## Lektion

```markdown
# Was ist eine Septime?
id: septime
for: any
needs: intervall
terms: septime

## Hören
Die **Septime** liegt einen *Ganzton* unter der [[oktave|Oktave]].
::: listen root=A steps="0 10" labels="1 b7"
```

- Kopf direkt unter dem Titel: `id` (gleich dem Ordnernamen), `for` (Standard `any`), `needs` (Lektionen davor, durch
  Leerzeichen getrennt), `terms` (Begriffe, die die Lektion lehrt). In beiden Sprachen gleich.
- `## Titel` beginnt eine Seite: ein Schritt auf dem Bildschirm, höchstens etwa ein Bildschirm Text.
- Text: Absätze, Listen (`- `, Fortsetzung mit zwei Leerzeichen), `**fett**`, `*kursiv*`, `[[id]]` oder
  `[[id|Text]]` für einen Glossar-Begriff.
- Beide Sprachen haben dieselben Seiten und Blöcke mit denselben Werten; nur der Text unterscheidet sich.

## Blöcke

| Block | Bedeutung |
|---|---|
| `::: figure root=A steps="0 7" labels="1 5"` | Töne am Instrument zeigen (Griffbrett, Klaviatur: das Profil entscheidet) |
| `::: listen root=A steps="0 7"` | Töne nacheinander vorspielen |
| `::: try root=A steps="0 7"` | nachspielen, Kontra hört zu; `octave=any`: jede Oktave zählt |
| `::: practice bundled=1 speeds="60 80 100"` | mitgelieferte Übung Nr. 1 (Nummer im Dateinamen von `testdata/exercises`) in drei Tempi (%) |
| `::: practice kind=groove variant=145 rhythm=quarters key=A bpm=70 judging=time` | eine Übung aus dem Generator |
| `::: tuner` | das Stimmgerät |
| `::: hint` … `:::` | Hinweis-Kasten (nur Klartext: keine Begriffe, kein Fett oder Kursiv) |
| `::: variant for=e-bass` … `:::` | Blöcke nur für diese Stufe |
| `::: question` … `:::` | Frage: Zeilen Frage, dann `- [x] richtig` und `- [ ] falsch` |
| `::: video src=media/a.webm` | Video, nur in nachgeladenen Kursen; mitgelieferte Kurse haben keine |

Tonhöhen stehen relativ: `root` ist ein Notenname (C, F#, Bb), `steps` sind Halbtöne darüber; Oktave und Lage wählt
das Profil: der Grundton liegt so tief wie möglich im Umfang des Instruments. `string=A` legt die Töne auf
Saiteninstrumenten auf die leere A-Saite (nur in `figure` und `try`, wo es darauf ankommt).

## Kurs

```json
{"id": "bass", "for": "bass", "chapters": [
  {"id": "loslegen", "de": "Loslegen", "en": "Getting started",
   "units": ["bass-teile", "stimmen"], "finale": "bundled=47 speeds=\"80 100\""}]}
```

Jede Lektion steht nach den Lektionen, die sie braucht (`needs`).

## Glossar

```markdown
## Septime
id: septime
lesson: septime
Die siebte Stufe über dem Grundton …
```

Ein Abschnitt je Begriff; Kopf `id`, `for`, `lesson` (die Lektion, die ihn lehrt). Blöcke wie in Lektionen.
