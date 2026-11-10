# Environment for builds in the VM (SYSTEM account): Rust, Git, LLVM, MSVC come from fixed places.
$env:CARGO_HOME = 'C:\KontraDev\cargo'; $env:RUSTUP_HOME = 'C:\KontraDev\rustup'
$env:CARGO_TARGET_DIR = 'C:\KontraDev\target'
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
$env:PATH = "C:\KontraDev\cargo\bin;C:\Program Files\Git\cmd;C:\Program Files\LLVM\bin;$env:PATH"
$env:CARGO_PROFILE_DEV_DEBUG = '0'; $env:CARGO_INCREMENTAL = '0'
$env:KONTRA_DATA_DIR = 'C:\KontraTest\data'
Set-Location C:\KontraDev\kontra
