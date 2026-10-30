
#ifndef AppVersion
  #define AppVersion "0.0.2"
#endif

#ifndef PackageDir
  #error PackageDir must be provided
#endif

#define AppName "DevInit"
#define AppExeName "devinit.exe"

[Setup]
AppId={{E3AE63AC-72DA-4D62-98B3-ED8C25F71B02}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=DevInit
DefaultDirName={localappdata}\Programs\DevInit
DefaultGroupName=DevInit
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputBaseFilename=DevInit-Setup-{#AppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#AppExeName}

[Files]
Source: "{#PackageDir}\devinit.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\resources\services\*.yaml"; DestDir: "{app}\resources\services"; Flags: ignoreversion
