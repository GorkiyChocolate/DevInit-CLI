
#ifndef AppVersion
  #define AppVersion "1.0.0-rc.2"
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
ChangesEnvironment=yes
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


[Code]
const
  EnvKey = 'Environment';
  DevInitKey = 'Software\DevInit';

function SamePath(const A, B: String): Boolean;
begin
  Result := CompareText(Trim(A), Trim(B)) = 0;
end;

function ContainsPath(const PathValue, Directory: String): Boolean;
var
  Remaining, Part: String;
  P: Integer;
begin
  Result := False;
  Remaining := PathValue;

  while Remaining <> '' do
  begin
    P := Pos(';', Remaining);

    if P = 0 then
    begin
      Part := Remaining;
      Remaining := '';
    end
    else
    begin
      Part := Copy(Remaining, 1, P - 1);
      Delete(Remaining, 1, P);
    end;

    if SamePath(Part, Directory) then
    begin
      Result := True;
      Exit;
    end;
  end;
end;

function RemovePath(const PathValue, Directory: String): String;
var
  Remaining, Part: String;
  P: Integer;
begin
  Result := '';
  Remaining := PathValue;

  while Remaining <> '' do
  begin
    P := Pos(';', Remaining);

    if P = 0 then
    begin
      Part := Remaining;
      Remaining := '';
    end
    else
    begin
      Part := Copy(Remaining, 1, P - 1);
      Delete(Remaining, 1, P);
    end;

    if (Trim(Part) <> '') and
       (not SamePath(Part, Directory)) then
    begin
      if Result <> '' then
        Result := Result + ';';

      Result := Result + Part;
    end;
  end;
end;

procedure InstallUserPath;
var
  CurrentPath, InstallPath, PreviousPath: String;
begin
  InstallPath := ExpandConstant('{app}');

  if not RegQueryStringValue(HKCU, EnvKey, 'Path', CurrentPath) then
    CurrentPath := '';

  if RegQueryStringValue(
    HKCU, DevInitKey, 'ManagedPath', PreviousPath
  ) then
  begin
    if not SamePath(PreviousPath, InstallPath) then
    begin
      CurrentPath := RemovePath(CurrentPath, PreviousPath);

      if not RegWriteStringValue(
        HKCU, EnvKey, 'Path', CurrentPath
      ) then
        RaiseException('Failed to update previous PATH');

      RegDeleteValue(HKCU, DevInitKey, 'ManagedPath');
    end;
  end;

  if not ContainsPath(CurrentPath, InstallPath) then
  begin
    if CurrentPath <> '' then
      CurrentPath := CurrentPath + ';';

    CurrentPath := CurrentPath + InstallPath;

    if not RegWriteStringValue(
      HKCU, EnvKey, 'Path', CurrentPath
    ) then
      RaiseException('Failed to update user PATH');

    if not RegWriteStringValue(
      HKCU, DevInitKey, 'ManagedPath', InstallPath
    ) then
      RaiseException('Failed to record DevInit PATH entry');
  end;
end;

procedure UninstallUserPath;
var
  CurrentPath, ManagedPath: String;
begin
  if not RegQueryStringValue(
    HKCU, DevInitKey, 'ManagedPath', ManagedPath
  ) then
    Exit;

  if RegQueryStringValue(HKCU, EnvKey, 'Path', CurrentPath) then
  begin
    CurrentPath := RemovePath(CurrentPath, ManagedPath);

    if not RegWriteStringValue(
      HKCU, EnvKey, 'Path', CurrentPath
    ) then
      RaiseException('Failed to remove DevInit from PATH');
  end;

  RegDeleteValue(HKCU, DevInitKey, 'ManagedPath');
  RegDeleteKeyIfEmpty(HKCU, DevInitKey);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    InstallUserPath;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    UninstallUserPath;
end;
