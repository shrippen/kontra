# Build tools for Kontra in the test VM: MSVC build tools, Rust (MSVC), LLVM (bindgen), Git.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = 'Tls12'
$dev = 'C:\KontraDev'
New-Item -ItemType Directory -Force $dev, "$dev\dl" | Out-Null
function Fetch($url, $file) {
    if (-not (Test-Path $file)) { Invoke-WebRequest $url -OutFile $file }
}

"== VS Build Tools"
if (-not (Test-Path 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC')) {
    Fetch 'https://aka.ms/vs/17/release/vs_BuildTools.exe' "$dev\dl\vs_BuildTools.exe"
    $p = Start-Process "$dev\dl\vs_BuildTools.exe" -Wait -PassThru -ArgumentList `
        '--quiet','--wait','--norestart','--nocache', `
        '--add','Microsoft.VisualStudio.Workload.VCTools','--includeRecommended'
    "vs_BuildTools exit $($p.ExitCode)"
}

"== LLVM"
if (-not (Test-Path 'C:\Program Files\LLVM\bin\libclang.dll')) {
    Fetch 'https://github.com/llvm/llvm-project/releases/download/llvmorg-19.1.7/LLVM-19.1.7-win64.exe' "$dev\dl\llvm.exe"
    $p = Start-Process "$dev\dl\llvm.exe" -Wait -PassThru -ArgumentList '/S'
    "llvm exit $($p.ExitCode)"
}

"== Git"
if (-not (Test-Path 'C:\Program Files\Git\cmd\git.exe')) {
    Fetch 'https://github.com/git-for-windows/git/releases/download/v2.47.1.windows.1/Git-2.47.1-64-bit.exe' "$dev\dl\git.exe"
    $p = Start-Process "$dev\dl\git.exe" -Wait -PassThru -ArgumentList '/VERYSILENT','/NORESTART','/SUPPRESSMSGBOXES'
    "git exit $($p.ExitCode)"
}

"== Rust"
$env:CARGO_HOME = "$dev\cargo"; $env:RUSTUP_HOME = "$dev\rustup"
if (-not (Test-Path "$dev\cargo\bin\cargo.exe")) {
    Fetch 'https://win.rustup.rs/x86_64' "$dev\dl\rustup-init.exe"
    $p = Start-Process "$dev\dl\rustup-init.exe" -Wait -PassThru -NoNewWindow -ArgumentList `
        '-y','--profile','minimal','--default-toolchain','stable-x86_64-pc-windows-msvc','-c','clippy','--no-modify-path'
    "rustup exit $($p.ExitCode)"
}
& "$dev\cargo\bin\rustc.exe" --version
Test-Path 'C:\Program Files\LLVM\bin\libclang.dll'
Test-Path 'C:\Program Files\Git\cmd\git.exe'
(Get-ChildItem 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC' -ErrorAction SilentlyContinue | Select -First 1).Name
"free GB: {0:N1}" -f ((Get-PSDrive C).Free/1GB)
