#define MyAppName "nanti"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "theepar"
#define MyAppURL "https://github.com/theepar/nanti-ide"
#define MyAppExeName "nanti.exe"

[Setup]
AppId={{5B2A3D18-7F4E-4390-B8A6-3E2C96C36154}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName}
UninstallDisplayName={#MyAppName}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\nanti
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline dialog
OutputDir=..\..\target\release
OutputBaseFilename=setup
SetupIconFile=..\..\crates\ui\assets\nanti.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesEnvironment=yes
DisableProgramGroupPage=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce
Name: "addcontextmenufiles"; Description: "Add ""Open with nanti"" to Windows Explorer file context menu"; GroupDescription: "Other:"
Name: "addcontextmenufolders"; Description: "Add ""Open with nanti"" to Windows Explorer directory context menu"; GroupDescription: "Other:"; Flags: checkedonce
Name: "addtopath"; Description: "Add to PATH (recommended)"; GroupDescription: "Other:"; Flags: checkedonce

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\crates\ui\assets\nanti.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\nanti.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\nanti.ico"; Tasks: desktopicon

[Registry]
; Context menu for files
Root: HKCU; Subkey: "Software\Classes\*\shell\nanti"; ValueType: string; ValueName: ""; ValueData: "Open with nanti"; Tasks: addcontextmenufiles; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\*\shell\nanti"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: addcontextmenufiles
Root: HKCU; Subkey: "Software\Classes\*\shell\nanti\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: addcontextmenufiles

; Context menu for directory
Root: HKCU; Subkey: "Software\Classes\Directory\shell\nanti"; ValueType: string; ValueName: ""; ValueData: "Open with nanti"; Tasks: addcontextmenufolders; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\shell\nanti"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: addcontextmenufolders
Root: HKCU; Subkey: "Software\Classes\Directory\shell\nanti\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%V"""; Tasks: addcontextmenufolders

; Context menu for directory background
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\nanti"; ValueType: string; ValueName: ""; ValueData: "Open with nanti"; Tasks: addcontextmenufolders; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\nanti"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: addcontextmenufolders
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\nanti\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%V"""; Tasks: addcontextmenufolders

; Add to PATH
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[UninstallDelete]
Type: filesandordirs; Name: "{app}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + UpperCase(Param) + ';', ';' + UpperCase(OrigPath) + ';') = 0;
end;

function CmdLineParamExists(Name: string): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), Name) = 0 then
    begin
      Result := True;
      exit;
    end;
end;

procedure RemoveFromPath(Dir: string);
var
  OrigPath: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
    exit;
  P := Pos(';' + UpperCase(Dir) + ';', ';' + UpperCase(OrigPath) + ';');
  if P = 0 then
    exit;
  if P = 1 then
    Delete(OrigPath, 1, Length(Dir) + 1)
  else
    Delete(OrigPath, P - 1, Length(Dir) + 1);
  RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath);
end;

function WantsDataRemoved(): Boolean;
begin
  if UninstallSilent then
    Result := CmdLineParamExists('/DELETEDATA')
  else
    Result := MsgBox('Also delete all nanti data on this computer?' + #13#10 + #13#10 +
      'This removes your settings, installed extensions, caches and the data folder:' + #13#10 +
      ExpandConstant('{userappdata}\nanti') + #13#10 + #13#10 +
      'Your own projects and files are never touched.', mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  DataDir: string;
begin
  if CurUninstallStep = usUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
  if CurUninstallStep = usPostUninstall then
  begin
    DataDir := ExpandConstant('{userappdata}\nanti');
    if DirExists(DataDir) and WantsDataRemoved() then
      DelTree(DataDir, True, True, True);
  end;
end;
