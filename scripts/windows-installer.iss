; =============================================================================
; Inno Setup 6 安装向导脚本（由 package-windows.ps1 -Installer 调用）
;
; 也可以手动编译：
;   "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" scripts\windows-installer.iss
;
; 可通过命令行 /D 覆盖的变量：
;   /DAppVersion=0.1.0
;   /DSourceExe=<exe 绝对路径>
;   /DOutputDir=<输出目录绝对路径>
; =============================================================================

#ifndef AppName
  #define AppName "GuessingGame"
#endif
#ifndef AppDisplayName
  #define AppDisplayName "猜数字"
#endif
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef AppPublisher
  #define AppPublisher "tanzhenxing"
#endif
#ifndef SourceExe
  #define SourceExe "..\dist\GuessingGame\GuessingGame.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\dist"
#endif

#define AppExeName "GuessingGame.exe"

[Setup]
; AppId 是升级/卸载的识别依据，必须保持恒定，不能随版本变化
AppId={{B7C1E0A4-3E4D-4C9A-9F2B-6A1D8C5E7F20}
AppName={#AppDisplayName}
AppVersion={#AppVersion}
AppVerName={#AppDisplayName} {#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppDisplayName}
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename={#AppName}-{#AppVersion}-setup
SetupIconFile=..\project\guessing-game\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
; 安装到 Program Files 需要管理员权限；改为 lowest 则会装到用户目录
PrivilegesRequired=admin

[Languages]
; Inno 7.x 起官方内置简体中文；6.x 使用社区翻译版（UTF-8 BOM）
#if VER >= EncodeVer(7,0,0)
Name: "chinesesimplified"; MessagesFile: "inno\ChineseSimplified-7.isl"
#else
Name: "chinesesimplified"; MessagesFile: "inno\ChineseSimplified-6.isl"
#endif

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppDisplayName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\{cm:UninstallProgram,{#AppDisplayName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppDisplayName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#AppDisplayName}}"; Flags: nowait postinstall skipifsilent
