# Windows-Testmaschine (libvirt-VM `win`)

Hilfen für Tests unter Windows in der VM des Eigentümers (Runde 18, Win2/Win6). Sie laufen auf
dem Linux-Rechner und sprechen über den QEMU-Gastagenten mit der VM; ein Login braucht es nicht.

```text
Host (Linux)                               Gast (Windows 11, Konto SYSTEM)
gexec.py '<PowerShell>'  ── guest-exec ──► powershell -EncodedCommand …  ─► Ausgabe zurück
gpush.py datei C:\ziel   ── guest-file-write (60 kB je Stück) ──► Datei
gpull.py C:\quelle datei ◄─ guest-file-read ──
```

- `install-tools.ps1` (einmal, im Gast): Visual Studio Build Tools, LLVM (für bindgen), Git und
  Rust (MSVC) nach `C:\KontraDev`.
- `dev.ps1` (im Gast, vor jedem Bauen mit `. C:\KontraDev\dev.ps1`): Pfade für cargo, Git, LLVM;
  Quelltext in `C:\KontraDev\kontra`, Build-Ordner `C:\KontraDev\target`.
- Änderungen kommen als Patch hinein: `git diff main > x.patch`, `gpush.py`, im Gast
  `git apply`. Der Host lässt keine Verbindungen aus der VM zu (Firewall), darum kein HTTP.
- Ein Fenster oder Ton im Desktop des angemeldeten Benutzers braucht eine geplante Aufgabe
  (`schtasks /create … /ru arian /it`, dann `/run`); die Ausgabe geht in eine Datei.
- Vor den ersten Tests liegt der Snapshot `vor-kontra` (nur Platte; ein voller Snapshot geht
  mit dieser Firmware nicht).
- Das Interface kommt per `virsh attach-device win <hostdev.xml> --live` in die VM (USB-ID
  des Scarlett Solo: `1235:8205`) und mit `detach-device` zurück.
- Mit ASIO bauen: `cargo build -p kontra-cli -p kontra-drivers --features kontra-drivers/asio`.
- Test am echten Gerät: `cargo test -p kontra-session --lib the_default_devices_can_monitor -- --ignored`.
