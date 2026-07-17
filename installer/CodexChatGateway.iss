#ifndef AppVersion
  #error AppVersion must be supplied by scripts/build-installer.ps1
#endif
#ifndef PayloadDir
  #error PayloadDir must be supplied by scripts/build-installer.ps1
#endif
#ifndef OutputDir
  #error OutputDir must be supplied by scripts/build-installer.ps1
#endif

#define AppName "Codex Chat Gateway"
#define AppExeName "CodexChatGateway.exe"
#define AppPublisher "codex-chat-gateway community"
#define AppUrl "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows"

[Setup]
AppId={{6B5FE367-E43A-46EE-B43A-A6117A5E6EF7}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
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
OutputBaseFilename=CodexChatGateway-Setup-v{#AppVersion}
SetupIconFile=..\desktop\assets\gateway-logo.ico
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
LZMAUseSeparateProcess=yes
InternalCompressLevel=ultra64
CloseApplications=yes
RestartApplications=no
AppMutex=Local\CodexChatGateway.Desktop.Singleton
SetupLogging=yes
DisableWelcomePage=no
WizardStyle=modern dark includetitlebar hidebevels
WizardSizePercent=120
WizardKeepAspectRatio=yes
WizardBackColor=#050807
WizardImageBackColor=#000000
WizardSmallImageBackColor=#050807
WizardImageFile=assets\installer-hero.png
WizardSmallImageFile=..\desktop\assets\gateway-logo.png
VersionInfoVersion={#AppVersion}.0
VersionInfoCompany={#AppPublisher}
VersionInfoDescription=Branded installer for Codex Chat Gateway
VersionInfoProductName={#AppName}
VersionInfoProductVersion={#AppVersion}
VersionInfoCopyright=MIT License

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "chinesesimplified"; MessagesFile: "ChineseSimplified.isl"

[CustomMessages]
english.SetupWindowTitle=Codex Chat Gateway Setup
english.WelcomeHeading=Your local model bridge, beautifully packaged.
english.WelcomeBody=Install the complete gateway, private runtime, and desktop console.%n%n• Listens only on 127.0.0.1%n• Keeps model keys on this computer%n• No separate Python, Docker, or WebView required
english.FinishHeading=Everything is ready.
english.FinishBody=Open the desktop console, add a model, and start the gateway when you are ready.
english.ShortcutGroup=Shortcuts
english.DesktopShortcut=Create a desktop shortcut
english.AutostartShortcut=Open the desktop console when I sign in
english.LaunchAfterInstall=Open Codex Chat Gateway
english.PurgePrompt=Also remove saved model keys, logs, and local settings?%n%nChoose No to keep them for a future reinstall.
english.SetupRunning=Codex Chat Gateway is currently open. Exit it from the tray menu, then run Setup again.
english.BrandKicker=LOCAL MODEL BRIDGE  /  WINDOWS
chinesesimplified.SetupWindowTitle=安装 Codex Chat Gateway
chinesesimplified.WelcomeHeading=把模型接入 Codex，从这里开始。
chinesesimplified.WelcomeBody=安装完整网关、独立运行时与桌面控制台。%n%n• 仅监听 127.0.0.1%n• 模型密钥只保存在本机%n• 无需另装 Python、Docker 或 WebView
chinesesimplified.FinishHeading=一切就绪。
chinesesimplified.FinishBody=打开桌面控制台，添加模型，然后按需启动本地网关。
chinesesimplified.ShortcutGroup=快捷方式
chinesesimplified.DesktopShortcut=创建桌面快捷方式
chinesesimplified.AutostartShortcut=登录 Windows 时打开桌面控制台
chinesesimplified.LaunchAfterInstall=打开 Codex Chat Gateway
chinesesimplified.PurgePrompt=是否同时删除已保存的模型密钥、日志和本地设置？%n%n选择“否”可保留这些数据，方便以后重新安装。
chinesesimplified.SetupRunning=Codex Chat Gateway 正在运行。请从托盘菜单退出后重新运行安装程序。
chinesesimplified.BrandKicker=LOCAL MODEL BRIDGE  /  WINDOWS

[Messages]
english.SetupAppRunningError={cm:SetupRunning}
chinesesimplified.SetupAppRunningError={cm:SetupRunning}

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopShortcut}"; GroupDescription: "{cm:ShortcutGroup}:"; Flags: unchecked
Name: "autostart"; Description: "{cm:AutostartShortcut}"; GroupDescription: "{cm:ShortcutGroup}:"; Flags: unchecked

[InstallDelete]
Type: filesandordirs; Name: "{app}\runtime"
Type: filesandordirs; Name: "{app}\scripts"
Type: filesandordirs; Name: "{app}\patches"

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: ".gateway\*,logs\*,*.pyc,__pycache__\*"
Source: "CHINESE_TRANSLATION_LICENSE.txt"; DestDir: "{app}\licenses"; DestName: "Inno-Setup-Chinese-Translation-MIT.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Codex Chat Gateway"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"
Name: "{autodesktop}\Codex Chat Gateway"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"; Tasks: desktopicon
Name: "{userstartup}\Codex Chat Gateway"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"; Tasks: autostart

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
  BrandKicker.AutoSize := True;
  BrandKicker.Left := WizardForm.WelcomeLabel1.Left;
  BrandKicker.Top := WizardForm.WelcomeLabel1.Top - ScaleY(28);
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
