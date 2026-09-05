#define MyAppName "ani-cli-rs"

[Setup]
AppId={{8A11B2C4-8E80-4E80-B462-92147321528A}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher="vorlie"
AppComments="Cross-platform Rust port of ani-cli"
PrivilegesRequired=lowest
DefaultDirName={userpf}\{#MyAppName}
DefaultGroupName={#MyAppName}
OutputDir={#OutputDir}
OutputBaseFilename={#MyAppName}-{#AppVersion}-windows-x64-setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes

[Files]
Source: "{#SourceDir}\ani-cli-rs.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\ani-cli-rs-gui.exe"; DestDir: "{app}"; Flags: ignoreversion; Check: FileExists(ExpandConstant('{#SourceDir}\ani-cli-rs-gui.exe'))
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Code]
// Automatically add {app} to User PATH on install
procedure CurStepChanged(CurStep: TSetupStep);
var
  OldPath: String;
  AppDir: String;
begin
  if CurStep = ssPostInstall then
  begin
    AppDir := ExpandConstant('{app}');
    if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OldPath) then
    begin
      if Pos(Uppercase(AppDir), Uppercase(OldPath)) = 0 then
      begin
        RegWriteStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OldPath + ';' + AppDir);
      end;
    end;
  end;
end;

// Remove {app} from User PATH on uninstall
procedure CurUninstallStepChanged(JustAfterAnUninstall: TUninstallStep);
var
  OldPath, AppDir: String;
  P: Integer;
begin
  if JustAfterAnUninstall = usPostUninstall then
  begin
    AppDir := ExpandConstant('{app}');
    if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OldPath) then
    begin
      P := Pos(';' + Uppercase(AppDir), Uppercase(OldPath));
      if P > 0 then
      begin
        Delete(OldPath, P, Length(AppDir) + 1);
        RegWriteStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OldPath);
      end
      else
      begin
        P := Pos(Uppercase(AppDir) + ';', Uppercase(OldPath));
        if P > 0 then
        begin
          Delete(OldPath, P, Length(AppDir) + 1);
          RegWriteStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OldPath);
        end;
      end;
    end;
  end;
end;
