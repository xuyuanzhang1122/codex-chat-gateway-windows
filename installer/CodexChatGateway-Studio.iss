; Codex Chat Gateway Studio (Tauri) branded installer
; Built by scripts/build-tauri-installer.ps1
#ifndef AppVersion
  #error AppVersion must be supplied by scripts/build-tauri-installer.ps1
#endif
#ifndef PayloadDir
  #error PayloadDir must be supplied by scripts/build-tauri-installer.ps1
#endif
#ifndef OutputDir
  #error OutputDir must be supplied by scripts/build-tauri-installer.ps1
#endif

#define AppName "Codex Chat Gateway"
#define AppExeName "CodexChatGateway.exe"
#define AppPublisher "xuyuanzhang1122 / codex-chat-gateway community"
#define AppUrl "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows"
; Keep the same product identity so upgrades replace the previous install folder.
#define AppIdGuid "{{6B5FE367-E43A-46EE-B43A-A6117A5E6EF7}"

[Setup]
AppId={#AppIdGuid}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} Studio {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}/issues
AppUpdatesURL={#AppUrl}/releases
DefaultDirName={localappdata}\Programs\Codex Chat Gateway
DefaultGroupName=Codex Chat Gateway
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=CodexChatGateway-Studio-Setup-v{#AppVersion}
SetupIconFile=..\desktop\assets\gateway-logo.ico
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma2/normal
SolidCompression=yes
LZMAUseSeparateProcess=yes
InternalCompressLevel=normal
CloseApplications=yes
RestartApplications=no
SetupLogging=yes
DisableWelcomePage=no
; Dark modern wizard (closest Inno ships to glass/studio chrome)
WizardStyle=modern dark includetitlebar hidebevels
WizardSizePercent=125
WizardKeepAspectRatio=yes
WizardBackColor=#07070b
WizardImageBackColor=#000000
WizardSmallImageBackColor=#07070b
WizardImageFile=assets\installer-hero.png
WizardSmallImageFile=..\desktop\assets\gateway-logo.png
VersionInfoVersion={#AppVersion}.0
VersionInfoCompany={#AppPublisher}
VersionInfoDescription=Studio installer for Codex Chat Gateway (Tauri)
VersionInfoProductName={#AppName} Studio
VersionInfoProductVersion={#AppVersion}
VersionInfoCopyright=MIT License
; Soft edge-ish: modern style already drops classic bevels
ShowLanguageDialog=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "chinesesimplified"; MessagesFile: "ChineseSimplified.isl"

[CustomMessages]
english.SetupWindowTitle=Codex Chat Gateway Studio Setup
english.WelcomeHeading=Studio console + local model bridge
english.WelcomeBody=Installs the Tauri desktop console, LiteLLM runtime, and scripts.%n%n• Listens only on 127.0.0.1%n• Model keys stay on this PC%n• Close-to-tray keeps the gateway running
english.FinishHeading=Studio is ready.
english.FinishBody=Open the console, add a model, then start the gateway. Closing the window hides to tray and does not stop the gateway.
english.ShortcutGroup=Shortcuts
english.DesktopShortcut=Create a desktop shortcut
english.AutostartShortcut=Start the gateway in the background when I sign in
english.LaunchAfterInstall=Open Codex Chat Gateway Studio
english.PurgePrompt=Also remove saved model keys, logs, and local settings?%n%nChoose No to keep them for a future reinstall.
english.SetupRunning=Codex Chat Gateway is currently open. Exit the console from the tray menu, then run Setup again.
english.BrandKicker=STUDIO  /  LOCAL MODEL BRIDGE
english.CleanupGroup=Previous installation
english.CleanPrevious=Remove previous program files and unrelated leftovers
english.CleanPreviousHint=Keeps .gateway model settings, API keys, .env and logs. The previous gateway is stopped so its runtime can be replaced safely.
chinesesimplified.SetupWindowTitle=安装 Codex Chat Gateway Studio
chinesesimplified.WelcomeHeading=Studio 控制台 · 本机模型桥
chinesesimplified.WelcomeBody=安装 Tauri 桌面控制台、LiteLLM 运行时与脚本。%n%n• 仅监听 127.0.0.1%n• 密钥只保存在本机%n• 关闭窗口仅到托盘，不停止网关
chinesesimplified.FinishHeading=Studio 已就绪。
chinesesimplified.FinishBody=打开控制台，添加模型并启动网关。关闭窗口会隐藏到托盘，不会停止网关进程。
chinesesimplified.ShortcutGroup=快捷方式
chinesesimplified.DesktopShortcut=创建桌面快捷方式
chinesesimplified.AutostartShortcut=登录 Windows 时在后台启动网关
chinesesimplified.LaunchAfterInstall=打开 Codex Chat Gateway Studio
chinesesimplified.PurgePrompt=是否同时删除已保存的模型密钥、日志和本地设置？%n%n选择“否”可保留这些数据，方便以后重新安装。
chinesesimplified.SetupRunning=Codex Chat Gateway 正在运行。请从托盘菜单退出控制台后重新运行安装程序。
chinesesimplified.BrandKicker=STUDIO  /  LOCAL MODEL BRIDGE
chinesesimplified.CleanupGroup=旧版本清理
chinesesimplified.CleanPrevious=删除以前版本的程序及无关文件
chinesesimplified.CleanPreviousHint=保留 .gateway 模型配置、API 密钥、.env 和日志。安装时会停止旧网关，以便安全替换运行时。

[Messages]
english.SetupAppRunningError={cm:SetupRunning}
chinesesimplified.SetupAppRunningError={cm:SetupRunning}

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopShortcut}"; GroupDescription: "{cm:ShortcutGroup}:"; Flags: unchecked
Name: "autostart"; Description: "{cm:AutostartShortcut}"; GroupDescription: "{cm:ShortcutGroup}:"; Flags: unchecked
Name: "cleanprevious"; Description: "{cm:CleanPrevious}"; GroupDescription: "{cm:CleanupGroup}:"; Flags: checkedonce

[InstallDelete]
; Replace runtime/scripts trees cleanly on upgrade
Type: filesandordirs; Name: "{app}\runtime"
Type: filesandordirs; Name: "{app}\scripts"
Type: filesandordirs; Name: "{app}\patches"; Check: ShouldCleanPreviousFiles
Type: filesandordirs; Name: "{app}\docs"; Check: ShouldCleanPreviousFiles
Type: filesandordirs; Name: "{app}\bin"; Check: ShouldCleanPreviousFiles
Type: filesandordirs; Name: "{app}\desktop"; Check: ShouldCleanPreviousFiles
Type: filesandordirs; Name: "{app}\examples"; Check: ShouldCleanPreviousFiles
Type: filesandordirs; Name: "{app}\installer"; Check: ShouldCleanPreviousFiles
Type: filesandordirs; Name: "{app}\tests"; Check: ShouldCleanPreviousFiles
Type: filesandordirs; Name: "{app}\__pycache__"; Check: ShouldCleanPreviousFiles
Type: files; Name: "{app}\*.bat"; Check: ShouldCleanPreviousFiles
Type: files; Name: "{app}\.gitignore"; Check: ShouldCleanPreviousFiles
Type: files; Name: "{app}\AGENTS.md"; Check: ShouldCleanPreviousFiles
Type: files; Name: "{app}\CHANGELOG.md"; Check: ShouldCleanPreviousFiles
Type: files; Name: "{app}\README.md"; Check: ShouldCleanPreviousFiles
Type: files; Name: "{app}\README.zh-CN.md"; Check: ShouldCleanPreviousFiles
Type: files; Name: "{app}\requirements.txt"; Check: ShouldCleanPreviousFiles
Type: files; Name: "{app}\CodexChatGateway.exe"
Type: files; Name: "{app}\codex-chat-gateway-desktop.exe"
Type: files; Name: "{app}\run_gateway.py"
Type: files; Name: "{app}\config.yaml"

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: ".gateway\*,logs\*,*.pyc,__pycache__\*,.env"
Source: "CHINESE_TRANSLATION_LICENSE.txt"; DestDir: "{app}\licenses"; DestName: "Inno-Setup-Chinese-Translation-MIT.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Codex Chat Gateway"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"
Name: "{autodesktop}\Codex Chat Gateway"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"; Tasks: desktopicon
Name: "{userstartup}\Codex Chat Gateway"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoLogo -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File ""{app}\scripts\start-background.ps1"" -NonInteractive"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"; Tasks: autostart

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchAfterInstall}"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\scripts\stop-background.ps1"""; Flags: runhidden waituntilterminated; RunOnceId: "StopGateway"; Check: FileExists(ExpandConstant('{app}\scripts\stop-background.ps1'))

[UninstallDelete]
Type: filesandordirs; Name: "{app}\.gateway"; Check: ShouldPurgeUserData
Type: filesandordirs; Name: "{app}\logs"; Check: ShouldPurgeUserData
Type: files; Name: "{app}\.env"; Check: ShouldPurgeUserData

[Code]
var
  BrandKicker: TNewStaticText;
  CleanupHint: TNewStaticText;
  PurgeUserData: Boolean;

procedure InitializeWizard;
begin
  WizardForm.Caption := ExpandConstant('{cm:SetupWindowTitle}');
  WizardForm.WelcomeLabel1.Caption := ExpandConstant('{cm:WelcomeHeading}');
  WizardForm.WelcomeLabel2.Caption := ExpandConstant('{cm:WelcomeBody}');
  WizardForm.FinishedHeadingLabel.Caption := ExpandConstant('{cm:FinishHeading}');
  WizardForm.FinishedLabel.Caption := ExpandConstant('{cm:FinishBody}');

  BrandKicker := TNewStaticText.Create(WizardForm);
  BrandKicker.Parent := WizardForm.WelcomePage;
  BrandKicker.Caption := ExpandConstant('{cm:BrandKicker}');
  BrandKicker.Font.Name := 'Consolas';
  BrandKicker.Font.Size := 8;
  BrandKicker.Font.Style := [fsBold];
  BrandKicker.Font.Color := $D47C8B; { purple-ish BGR for accent }
  BrandKicker.AutoSize := True;
  BrandKicker.Left := WizardForm.WelcomeLabel1.Left;
  BrandKicker.Top := WizardForm.WelcomeLabel1.Top - ScaleY(28);

  CleanupHint := TNewStaticText.Create(WizardForm);
  CleanupHint.Parent := WizardForm.SelectTasksPage;
  CleanupHint.Caption := ExpandConstant('{cm:CleanPreviousHint}');
  CleanupHint.Font.Name := 'Segoe UI';
  CleanupHint.Font.Size := 8;
  CleanupHint.WordWrap := True;
  CleanupHint.AutoSize := False;
  CleanupHint.Width := WizardForm.TasksList.Width;
  CleanupHint.Height := ScaleY(40);
  CleanupHint.Left := WizardForm.TasksList.Left;
  CleanupHint.Top := WizardForm.TasksList.Top + WizardForm.TasksList.Height + ScaleY(8);
end;

procedure StopPreviousProcesses;
var
  ResultCode: Integer;
begin
  { Prefer the scoped stop script from the installed copy. }
  if FileExists(ExpandConstant('{app}\scripts\stop-background.ps1')) then
    Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      '-NoLogo -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File "' +
      ExpandConstant('{app}\scripts\stop-background.ps1') + '"',
      ExpandConstant('{app}'), SW_HIDE, ewWaitUntilTerminated, ResultCode);
  { Best-effort cleanup for old desktop editions. }
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM CodexChatGateway.exe /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM codex-chat-gateway-desktop.exe /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

function ShouldCleanPreviousFiles: Boolean;
begin
  Result := WizardIsTaskSelected('cleanprevious');
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if WizardIsTaskSelected('cleanprevious') then
    StopPreviousProcesses;
  NeedsRestart := False;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    if UninstallSilent then
      PurgeUserData := False
    else
      PurgeUserData := MsgBox(ExpandConstant('{cm:PurgePrompt}'), mbConfirmation, MB_YESNO) = IDYES;
  end;
end;

function ShouldPurgeUserData: Boolean;
begin
  Result := PurgeUserData;
end;
