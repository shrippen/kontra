; Installer of Kontra for Windows (Win5, LH-NF-17): wraps the folder scripts/package-windows.sh
; builds. Per user, no administrator rights (P8): the program goes to
; %LOCALAPPDATA%\Programs\Kontra, the Start menu entry and the uninstaller are the user's own.
; Uninstalling removes the program, never %APPDATA%\Kontra (pieces, progress, settings).
; Built by scripts/installer-windows.sh, which passes the three defines:
;   iscc /DAppVersion=0.1.0 /DSourceDir=C:\...\dist\Kontra /DOutputDir=C:\...\out Kontra.iss

#ifndef AppVersion
  #error AppVersion is not defined (iscc /DAppVersion=X.Y.Z)
#endif
#ifndef SourceDir
  #error SourceDir is not defined (the folder of package-windows.sh)
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

#define AppName "Kontra"
#define AppExe "kontra.exe"

[Setup]
; Fixed for good: a new installer finds the old one by it and updates in place.
AppId={{6C1B6F3A-6B0E-4C8A-9E52-7D0F1B7A4C21}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=shrippen
AppPublisherURL=https://shrippen.github.io/
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; The package is GPLv3 (ASIO) and carries Qt and KDE Frameworks under the LGPL: the user reads
; that notice before installing; the full texts are in the installed licence folder.
LicenseFile={#SourceDir}\share\licenses\Kontra\NOTICE.txt
OutputDir={#OutputDir}
OutputBaseFilename=Kontra-{#AppVersion}-windows-setup
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\bin\{#AppExe}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\bin\{#AppExe}"

[Run]
Filename: "{app}\bin\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent
