#ifndef AppVersion
#define AppVersion "0.2.0"
#endif

[Setup]
AppId={{F4F132D2-9B37-4C03-95D5-362E83531990}}
AppName=Potoo IDE
AppVersion={#AppVersion}
AppPublisher=Diego Pereira
AppPublisherURL=https://github.com/diegopn/potoo-ide
AppSupportURL=https://github.com/diegopn/potoo-ide/issues
AppUpdatesURL=https://github.com/diegopn/potoo-ide/releases
DefaultDirName={autopf}\Potoo IDE
DefaultGroupName=Potoo IDE
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=lowest
OutputDir=..\..\target\packages\windows
OutputBaseFilename=Potoo-IDE-{#AppVersion}-windows-x64-setup
SetupIconFile=..\..\data\icons\windows\io.github.diegopn.potooide.ico
UninstallDisplayIcon={app}\potoo-ide.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\..\LICENSE

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\..\target\packages\windows\stage\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{autoprograms}\Potoo IDE"; Filename: "{app}\bin\potoo-ide.exe"; WorkingDir: "{app}\bin"; IconFilename: "{app}\potoo-ide.ico"
Name: "{autodesktop}\Potoo IDE"; Filename: "{app}\bin\potoo-ide.exe"; WorkingDir: "{app}\bin"; IconFilename: "{app}\potoo-ide.ico"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Run]
Filename: "{app}\bin\potoo-ide.exe"; Description: "Launch Potoo IDE"; Flags: postinstall nowait skipifsilent
