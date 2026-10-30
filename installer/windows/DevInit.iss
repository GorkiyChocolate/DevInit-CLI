
#ifndef AppVersion
  #define AppVersion "0.0.2"
#endif

#ifndef BinaryDir
  #error BinaryDir must be provided
#endif

#ifndef RegistryDir
  #error RegistryDir must be provided
#endif

#ifndef OutputDir
  #define OutputDir "dist"
#endif

#define AppName "DevInit"
#define AppExeName "devinit.exe"

[Setup]
AppId={{E3AE63AC-72DA-4D62-98B3-ED8C25F71B02}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=DevInit
DefaultDirName={autopf}\DevInit
DefaultGroupName=DevInit
OutputDir={#OutputDir}
OutputBaseFilename=DevInit-Setup-{#AppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesEnvironment=yes
UninstallDisplayIcon={app}\{#AppExeName}

[Files]
Source: "{#BinaryDir}\devinit.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RegistryDir}\*.yaml"; DestDir: "{app}\resources\services"; Flags: ignoreversion

[Tasks]
Name: "addtopath"; Description: "Add DevInit to PATH"; Flags: checkedonce

[Registry]
Root: HKA; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Code]
function NeedsAddPath(Param: string): Boolean;
var
  CurrentPath: string;
begin
  if not RegQueryStringValue(HKA, 'Environment', 'Path', CurrentPath) then
    CurrentPath := '';
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(CurrentPath) + ';') = 0;
end;
