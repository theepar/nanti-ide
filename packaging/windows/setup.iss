#define MyAppName "Nanti IDE"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "theepar"
#define MyAppURL "https://github.com/theepar/nanti-ide"
#define MyAppExeName "Nanti IDE.exe"

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
DefaultDirName={autopf}\Nanti IDE
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline dialog
OutputDir=..\..\target\release
OutputBaseFilename=Nanti-IDE-1.0.0-setup
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
Name: "addcontextmenufiles"; Description: "Add ""Open with Nanti IDE"" to Windows Explorer file context menu"; GroupDescription: "Other:"
Name: "addcontextmenufolders"; Description: "Add ""Open with Nanti IDE"" to Windows Explorer directory context menu"; GroupDescription: "Other:"; Flags: checkedonce
Name: "addtopath"; Description: "Add to PATH (recommended)"; GroupDescription: "Other:"; Flags: checkedonce

[Files]
Source: "..\..\target\release\nanti.exe"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: ignoreversion
Source: "..\..\target\release\nanti.exe"; DestDir: "{app}"; DestName: "nanti.exe"; Flags: ignoreversion
Source: "..\..\crates\ui\assets\nanti.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\runtime\node.exe"; DestDir: "{app}\runtime"; Flags: ignoreversion
Source: "..\..\target\release\runtime\git\*"; DestDir: "{app}\runtime\git"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "runtime-notice.txt"; DestDir: "{app}\runtime"; Flags: ignoreversion
Source: "..\..\target\release\runtime\MicrosoftEdgeWebview2Setup.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall; Check: NeedsWebView2

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\nanti.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\nanti.ico"; Tasks: desktopicon

[Registry]
; Context menu for files
Root: HKCU; Subkey: "Software\Classes\*\shell\nanti"; ValueType: string; ValueName: ""; ValueData: "Open with Nanti IDE"; Tasks: addcontextmenufiles; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\*\shell\nanti"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: addcontextmenufiles
Root: HKCU; Subkey: "Software\Classes\*\shell\nanti\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: addcontextmenufiles

; Context menu for directory
Root: HKCU; Subkey: "Software\Classes\Directory\shell\nanti"; ValueType: string; ValueName: ""; ValueData: "Open with Nanti IDE"; Tasks: addcontextmenufolders; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\shell\nanti"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: addcontextmenufolders
Root: HKCU; Subkey: "Software\Classes\Directory\shell\nanti\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%V"""; Tasks: addcontextmenufolders

; Context menu for directory background
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\nanti"; ValueType: string; ValueName: ""; ValueData: "Open with Nanti IDE"; Tasks: addcontextmenufolders; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\nanti"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: addcontextmenufolders
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\nanti\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%V"""; Tasks: addcontextmenufolders

; Add to PATH
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[UninstallDelete]
Type: filesandordirs; Name: "{app}"

[Run]
Filename: "{tmp}\MicrosoftEdgeWebview2Setup.exe"; Parameters: "/silent /install"; StatusMsg: "Installing the WebView2 runtime..."; Flags: waituntilterminated; Check: NeedsWebView2
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
function WebView2Version(Root: Integer; Key: string): string;
begin
  if not RegQueryStringValue(Root, Key, 'pv', Result) then
    Result := '';
end;

function NeedsWebView2: boolean;
var
  Found, Clients: string;
begin
  Clients := '\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  Found := WebView2Version(HKLM, 'SOFTWARE\WOW6432Node' + Clients);
  if Found = '' then Found := WebView2Version(HKLM, 'SOFTWARE' + Clients);
  if Found = '' then Found := WebView2Version(HKCU, 'Software' + Clients);
  Result := (Found = '') or (Found = '0.0.0.0');
end;

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
