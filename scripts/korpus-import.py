#!/usr/bin/env python3
"""Converts annotated bass datasets into Kontra's corpus format (testdata/korpus/QUELLEN.md).

For every item it writes `<name>.wav` (mono, 16 bit) and `<name>.json`:

    {"set": "Line4", "sample_rate": 44100,
     "notes": [{"onset_s": 0.5, "midi": 43.0}, ...],
     "meta": {...}}

`set`, `sample_rate` and `notes` match `KorpusItem` in crates/engine/src/korpus.rs; serde ignores `meta`
(dataset, licence, recording chain and per-note string/fret/technique where the dataset has them).

Only the standard library is used; MP3 input (FiloBass) is decoded with the `ffmpeg` binary.
Output belongs in testdata/korpus/local/ (gitignored): the datasets' licences do not allow committing them.

Usage:
    korpus-import.py idmt-st  SRC_DIR OUT_DIR [--pad 0.5] [--normalize -6] [--exclude-expr HA,DN]
    korpus-import.py idmt-sb  SRC_DIR OUT_DIR   (writes OUT_DIR/bass-1 … bass-3)
    korpus-import.py filobass SRC_DIR OUT_DIR [--max-seconds 60] [--limit 6]
    korpus-import.py nsynth   SRC_DIR OUT_DIR [--min-midi 23 --max-midi 67]
    korpus-import.py csv      WAV CSV OUT_DIR --name NAME --set Line4   (CSV: onset_s,midi per line)
"""

import argparse
import array
import csv
import json
import os
import struct
import subprocess
import sys
import tempfile
import wave
import xml.etree.ElementTree as ET

SETS = ("Line4", "Line5", "LaptopJack4", "Fretless5", "Sixteenths")


# --- audio -------------------------------------------------------------------------------------


def read_wav_mono(path, max_seconds=None):
    """Returns (samples as list of floats in -1..1, sample_rate). PCM 16/24/32 bit, any channel count."""
    with wave.open(path, "rb") as w:
        ch, width, sr, n = w.getnchannels(), w.getsampwidth(), w.getframerate(), w.getnframes()
        if max_seconds:
            n = min(n, int(max_seconds * sr))
        raw = w.readframes(n)
    if width == 2:
        data = array.array("h", raw)
        scale = 32768.0
    elif width == 4:
        data = array.array("i", raw)
        scale = 2147483648.0
    elif width == 3:
        data = [int.from_bytes(raw[i : i + 3], "little", signed=True) for i in range(0, len(raw), 3)]
        scale = 8388608.0
    elif width == 1:
        data = [b - 128 for b in raw]
        scale = 128.0
    else:
        raise ValueError(f"{path}: unsupported sample width {width}")
    if sys.byteorder == "big" and isinstance(data, array.array):
        data.byteswap()
    if ch == 1:
        return [s / scale for s in data], sr
    frames = len(data) // ch
    return [sum(data[i * ch : i * ch + ch]) / (ch * scale) for i in range(frames)], sr


def decode_with_ffmpeg(path, max_seconds=None):
    """Decodes any format ffmpeg reads to mono 16-bit WAV and reads it."""
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "x.wav")
        cmd = ["ffmpeg", "-nostdin", "-loglevel", "error", "-i", path, "-ac", "1", "-c:a", "pcm_s16le"]
        if max_seconds:
            cmd += ["-t", str(max_seconds)]
        subprocess.run(cmd + [out], check=True)
        return read_wav_mono(out)


def write_wav_mono(path, samples, sr):
    pcm = array.array("h", (int(max(-1.0, min(1.0, s)) * 32767) for s in samples))
    if sys.byteorder == "big":
        pcm.byteswap()
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm.tobytes())


NORMALIZE_DB = None  # set from --normalize


def write_item(out_dir, name, samples, sr, notes, set_name, meta, pad_s=0.0):
    """Writes one item; `pad_s` seconds of silence go in front and shift every onset."""
    if set_name not in SETS:
        raise ValueError(f"unknown set {set_name}")
    if NORMALIZE_DB is not None:
        peak = max((abs(s) for s in samples), default=0.0)
        if peak > 0:
            gain = 10 ** (NORMALIZE_DB / 20) / peak
            samples = [s * gain for s in samples]
            meta = dict(meta, normalized_peak_db=NORMALIZE_DB)
    pad = [0.0] * int(round(pad_s * sr))
    notes = sorted(notes, key=lambda n: n["onset_s"])
    body = {
        "set": set_name,
        "sample_rate": sr,
        "notes": [
            dict(
                {"onset_s": round(n["onset_s"] + pad_s, 6), "midi": float(n["midi"])},
                **({"technique": n["technique"]} if n.get("technique") else {}),
                **({"slide_cents": n["slide_cents"]} if n.get("slide_cents") else {}),
            )
            for n in notes
        ],
        "meta": dict(meta, pad_s=pad_s, note_meta=[n.get("meta", {}) for n in notes]),
    }
    os.makedirs(out_dir, exist_ok=True)
    write_wav_mono(os.path.join(out_dir, name + ".wav"), pad + list(samples), sr)
    with open(os.path.join(out_dir, name + ".json"), "w") as f:
        json.dump(body, f, indent=1)
    print(f"{name}: {len(notes)} notes, {len(samples) / sr:.1f} s, set {set_name}")


# --- MIDI (format 0/1, notes only) -------------------------------------------------------------


def read_midi_notes(path):
    """Returns [(onset_s, pitch, channel)] for every note-on (velocity > 0), honouring tempo changes."""
    with open(path, "rb") as f:
        data = f.read()
    if data[:4] != b"MThd":
        raise ValueError(f"{path}: no MIDI header")
    hlen = struct.unpack(">I", data[4:8])[0]
    _fmt, ntracks, division = struct.unpack(">HHH", data[8:14])
    if division & 0x8000:
        raise ValueError(f"{path}: SMPTE time division not supported")
    pos = 8 + hlen
    events = []  # (tick, order, kind, value...)
    for _ in range(ntracks):
        if data[pos : pos + 4] != b"MTrk":
            break
        tlen = struct.unpack(">I", data[pos + 4 : pos + 8])[0]
        p, end = pos + 8, pos + 8 + tlen
        tick, status = 0, 0
        while p < end:
            delta = 0
            while True:
                b = data[p]
                p += 1
                delta = (delta << 7) | (b & 0x7F)
                if not b & 0x80:
                    break
            tick += delta
            b = data[p]
            if b & 0x80:
                status = b
                p += 1
            if status == 0xFF:
                mtype = data[p]
                p += 1
                ln = 0
                while True:
                    c = data[p]
                    p += 1
                    ln = (ln << 7) | (c & 0x7F)
                    if not c & 0x80:
                        break
                if mtype == 0x51:
                    events.append((tick, 0, "tempo", int.from_bytes(data[p : p + 3], "big")))
                p += ln
            elif status in (0xF0, 0xF7):
                ln = 0
                while True:
                    c = data[p]
                    p += 1
                    ln = (ln << 7) | (c & 0x7F)
                    if not c & 0x80:
                        break
                p += ln
            else:
                kind = status & 0xF0
                nbytes = 1 if kind in (0xC0, 0xD0) else 2
                args = data[p : p + nbytes]
                p += nbytes
                if kind == 0x90 and args[1] > 0:
                    events.append((tick, 1, "note", args[0], status & 0x0F))
        pos = end
    events.sort(key=lambda e: (e[0], e[1]))
    tempo, last_tick, seconds, notes = 500000, 0, 0.0, []
    for e in events:
        seconds += (e[0] - last_tick) * tempo / 1e6 / division
        last_tick = e[0]
        if e[2] == "tempo":
            tempo = e[3]
        else:
            notes.append((seconds, e[3], e[4]))
    return notes


# --- datasets ----------------------------------------------------------------------------------


# IDMT styles as Kontra's techniques (LH-BEW-06): slap thumb, slap pluck (pop), dead note (ghost),
# harmonic (its pitch counts in any octave). Finger, pick and palm mute are plain plucks;
# vibrato and bend are not told.
IDMT_TECHNIQUES = {"ST": "Slap", "SP": "Pop", "DN": "Muted", "HA": "Harmonic"}
# An IDMT slide (SL) is not a technique of the annotated note: the note is plucked and slides
# 150-350 ms later fret by fret over its range (modulationFrequencyRange, 100 or 200 cents) to a
# pitch IDMT does not annotate. Kontra hears the slide where it arrives; `slide_cents` lets
# `kontra-cli korpus techniques` look for it there.
IDMT_SLIDE = "SL"


def import_idmt_st(src, out, args):
    """IDMT-SMT-Bass-Single-Track: 17 lines, 4-string electric bass, XML note annotations."""
    for fn in sorted(os.listdir(os.path.join(src, "annotation"))):
        if not fn.endswith(".xml"):
            continue
        root = ET.parse(os.path.join(src, "annotation", fn)).getroot()
        g = root.find("globalParameter")
        notes = []
        for e in root.findall("transcription/event"):
            pluck, expr = e.findtext("excitationStyle"), e.findtext("expressionStyle")
            notes.append(
                {
                    "onset_s": float(e.findtext("onsetSec")),
                    "midi": float(e.findtext("pitch")),
                    "technique": IDMT_TECHNIQUES.get(expr) or IDMT_TECHNIQUES.get(pluck),
                    "slide_cents": float(e.findtext("modulationFrequencyRange")) if expr == IDMT_SLIDE else None,
                    "meta": {
                        "offset_s": float(e.findtext("offsetSec")),
                        "string": int(e.findtext("stringNumber")),
                        "fret": int(e.findtext("fretNumber")),
                        "pluck": e.findtext("excitationStyle"),
                        "expr": e.findtext("expressionStyle"),
                    },
                }
            )
        if args.exclude_expr:
            notes = [n for n in notes if n["meta"]["expr"] not in args.exclude_expr.split(",")]
        samples, sr = read_wav_mono(os.path.join(src, "audio", g.findtext("audioFileName")))
        meta = {
            "dataset": "IDMT-SMT-Bass-Single-Track",
            "licence": "CC BY-NC-ND 4.0 per Zenodo 7544099 (PDF in archive says CC BY-SA 4.0); local use only",
            "instrument": g.findtext("instrumentModel"),
            "chain": "electric bass, clean line signal (no amp/room audible); onsets are score-grid times",
        }
        write_item(out, "idmt-st-" + fn[:-4], samples, sr, notes, args.set or "Line4", meta, args.pad)


def import_filobass(src, out, args):
    """FiloBass: jazz double bass, Demucs-separated stems (MP3) + performance-aligned MIDI."""
    midi_dir = os.path.join(src, args.midi_variant)
    names = sorted(f[:-4] for f in os.listdir(midi_dir) if f.endswith(".mid"))
    if args.limit:
        names = names[: args.limit]
    for name in names:
        mp3 = os.path.join(src, "audio_bass_stems", name + ".mp3")
        if not os.path.exists(mp3):
            continue
        samples, sr = decode_with_ffmpeg(mp3, args.max_seconds)
        dur = len(samples) / sr
        notes = [
            {"onset_s": t, "midi": p + args.transpose}
            for t, p, _ch in read_midi_notes(os.path.join(midi_dir, name + ".mid"))
            if t < dur - 0.2
        ]
        meta = {
            "dataset": "FiloBass (" + args.midi_variant + ")",
            "licence": "annotations CC BY 4.0; audio separated from Aebersold play-alongs, local use only",
            "instrument": "double bass (jazz, walking)",
            "chain": "studio recording, Demucs-separated MP3 stem",
        }
        write_item(out, "filobass-" + name, samples, sr, notes, args.set or "Line4", meta, args.pad)


def import_nsynth(src, out, args):
    """NSynth: 4 s single notes from sample libraries, onset at 0. Concatenated per instrument."""
    with open(os.path.join(src, "examples.json")) as f:
        examples = json.load(f)
    by_inst = {}
    for key, v in examples.items():
        if v["instrument_family_str"] != "bass" or v["instrument_source_str"] not in args.sources.split(","):
            continue
        if not args.min_midi <= v["pitch"] <= args.max_midi:
            continue
        by_inst.setdefault(v["instrument_str"], []).append(v)
    gap = 0.5
    for inst, items in sorted(by_inst.items()):
        items.sort(key=lambda v: (v["pitch"], v["velocity"]))
        samples, notes, sr = [], [], None
        for v in items:
            s, rate = read_wav_mono(os.path.join(src, "audio", v["note_str"] + ".wav"))
            sr = sr or rate
            samples += [0.0] * int(gap * sr)
            notes.append({"onset_s": len(samples) / sr, "midi": float(v["pitch"]), "meta": {"velocity": v["velocity"]}})
            samples += s
        low = min(n["midi"] for n in notes)
        meta = {
            "dataset": "NSynth test (" + inst + ")",
            "licence": "CC BY 4.0 (redistributable with attribution)",
            "instrument": inst,
            "chain": "sample library one-shots, 16 kHz",
        }
        write_item(out, "nsynth-" + inst, samples, sr, notes, args.set or ("Line5" if low < 28 else "Line4"), meta, 0.0)


# IDMT-SMT-Bass (single notes): BS_<bass>_EQ_<pickup>_<pluck>_<expr>_<string>_<fret>.wav, string 1 = E.
# Only the folders whose notes Kontra tells, or must leave plain (palm mute, vibrato, bend).
IDMT_SB_OPEN_MIDI = {1: 28, 2: 33, 3: 38, 4: 43}
IDMT_SB_FOLDERS = {
    "PS/FS": None,
    "PS/PK": None,
    "PS/MU": "PalmMute",
    "PS/ST": "Slap",
    "PS/SP": "Pop",
    "ES/DN": "Muted",
    "ES/VI": "Vibrato",
    "ES/BE": "Bend",
}
# Bass 1 is the bass of IDMT-SMT-Bass-Single-Track (description PDF in the archive).
IDMT_SB_BASSES = {1: "Fame Baphomet NTB 4", 2: "Warwick 4", 3: "Yamaha BB604 Jazz Bass 4"}
IDMT_SB_NOTE_S = 1.5
IDMT_SB_GAP_S = 0.5
IDMT_SB_FADE_S = 0.03
IDMT_SB_ONSET_SHARE = 0.05
IDMT_SB_BLOCK_S = 0.005


def idmt_sb_attack(s, sr):
    """Start of the attack that reaches the file's peak: back from the loudest 5-ms block while blocks
    stay above 5 % of the peak, then the first sample above 5 % in that block. Many files have a soft
    touch of the string 100-400 ms before the pluck (5-10 % of the peak); the first sample above 5 % of
    the file would put the onset there."""
    block = max(1, int(IDMT_SB_BLOCK_S * sr))
    env = [max((abs(x) for x in s[i : i + block]), default=0.0) for i in range(0, len(s), block)]
    if not env:
        return 0
    b = max(range(len(env)), key=env.__getitem__)
    floor = IDMT_SB_ONSET_SHARE * env[b]
    while b > 0 and env[b - 1] > floor:
        b -= 1
    return next((i for i in range(b * block, len(s)) if abs(s[i]) > floor), b * block)


def idmt_sb_name(fn):
    """(bass, pickup, string, fret) from a file name, None for other files."""
    p = fn[:-4].split("_")
    if not fn.endswith(".wav") or len(p) != 8 or p[0] != "BS" or not all(p[i].isdigit() for i in (1, 3, 6, 7)):
        return None
    return int(p[1]), int(p[3]), int(p[6]), int(p[7])


def import_idmt_sb(src, out, args):
    """IDMT-SMT-Bass: single notes of three basses, one WAV each. Concatenated per bass and folder into
    OUT/bass-<n>/, each note cut to 1.5 s from its attack (`idmt_sb_attack`) with a short fade, after
    0.5 s of silence."""
    for folder, technique in IDMT_SB_FOLDERS.items():
        by_bass = {}
        for fn in sorted(os.listdir(os.path.join(src, folder))):
            parsed = idmt_sb_name(fn)
            if parsed is None or parsed[0] not in IDMT_SB_BASSES or parsed[2] not in IDMT_SB_OPEN_MIDI:
                continue
            by_bass.setdefault(parsed[0], []).append((parsed[2], parsed[3], parsed[1], fn))
        for bass, files in sorted(by_bass.items()):
            samples, notes, sr = [], [], None
            for string, fret, pickup, fn in sorted(files):
                s, rate = read_wav_mono(os.path.join(src, folder, fn), IDMT_SB_NOTE_S + 1.0)
                if sr not in (None, rate):
                    raise ValueError(f"{fn}: sample rate {rate}, expected {sr}")
                sr = rate
                start = idmt_sb_attack(s, sr)
                s = s[start : start + int(IDMT_SB_NOTE_S * sr)]
                fade = int(IDMT_SB_FADE_S * sr)
                for k in range(min(fade, len(s))):
                    s[-1 - k] *= k / fade
                samples += [0.0] * int(IDMT_SB_GAP_S * sr)
                notes.append(
                    {
                        "onset_s": len(samples) / sr,
                        "midi": float(IDMT_SB_OPEN_MIDI[string] + fret),
                        "technique": technique,
                        "meta": {"string": string, "fret": fret, "pickup": pickup, "file": fn},
                    }
                )
                samples += s
            meta = {
                "dataset": "IDMT-SMT-Bass (Zenodo 7188892), folder " + folder,
                "licence": "CC BY-NC-ND 4.0 per Zenodo (PDF in archive says CC BY-SA 4.0); local use only",
                "instrument": IDMT_SB_BASSES[bass],
                "chain": "electric bass, line signal, single notes concatenated",
            }
            name = "idmt-sb-" + folder.replace("/", "-").lower()
            write_item(os.path.join(out, f"bass-{bass}"), name, samples, sr, notes, args.set or "Line4", meta, 0.0)


def import_csv(wav, csv_path, out, args):
    samples, sr = read_wav_mono(wav) if wav.lower().endswith(".wav") else decode_with_ffmpeg(wav)
    with open(csv_path) as f:
        notes = [{"onset_s": float(r[0]), "midi": float(r[1])} for r in csv.reader(f) if r and not r[0].startswith("#")]
    write_item(out, args.name, samples, sr, notes, args.set or "Line4", {"dataset": "csv"}, args.pad)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("dataset", choices=["idmt-st", "idmt-sb", "filobass", "nsynth", "csv"])
    ap.add_argument("paths", nargs="+", help="SRC OUT (csv: WAV CSV OUT)")
    ap.add_argument("--set", choices=SETS, help="KorpusSet written into the JSON (default per dataset)")
    ap.add_argument("--pad", type=float, default=0.5, help="silence in seconds before the audio (default 0.5)")
    ap.add_argument("--normalize", type=float, metavar="DB", help="scale each item to this peak level, e.g. -6")
    ap.add_argument("--name", default="item", help="csv: output name")
    ap.add_argument("--exclude-expr", default="", help="idmt-st: skip expression styles, e.g. HA,DN")
    ap.add_argument("--midi-variant", default="midi_fully_aligned", help="filobass: MIDI folder")
    ap.add_argument("--transpose", type=int, default=0, help="filobass: semitones added to MIDI pitch")
    ap.add_argument("--max-seconds", type=float, help="filobass: crop each track")
    ap.add_argument("--limit", type=int, help="filobass: number of tracks")
    ap.add_argument("--sources", default="electronic,acoustic", help="nsynth: instrument sources")
    ap.add_argument("--min-midi", type=int, default=23)
    ap.add_argument("--max-midi", type=int, default=67)
    args = ap.parse_args()
    global NORMALIZE_DB
    NORMALIZE_DB = args.normalize
    if args.dataset == "csv":
        if len(args.paths) != 3:
            ap.error("csv needs WAV CSV OUT")
        import_csv(*args.paths, args)
        return
    if len(args.paths) != 2:
        ap.error("needs SRC OUT")
    src, out = args.paths
    importers = {"idmt-st": import_idmt_st, "idmt-sb": import_idmt_sb, "filobass": import_filobass, "nsynth": import_nsynth}
    importers[args.dataset](src, out, args)


if __name__ == "__main__":
    main()
