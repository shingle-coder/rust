# =============================================================================
# 一键打包 Windows 客户端
#
# 必须在 Windows 上运行（Rust 的 GUI 依赖无法从 macOS/Linux 直接交叉编译出 Windows 程序）。
# 用法（在仓库根目录打开 PowerShell）：
#   .\scripts\package-windows.ps1             # 生成 dist\GuessingGame\GuessingGame.exe
#   .\scripts\package-windows.ps1 -Zip        # 额外生成 zip 压缩包
#   .\scripts\package-windows.ps1 -Installer  # 额外生成 setup.exe 安装向导
#
# -Installer 需要先安装 Inno Setup 6（并提供 ISCC.exe）：
#   winget install JRSoftware.InnoSetup
#
# 如果提示禁止执行脚本，先运行：
#   Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
# =============================================================================
param(
    [switch]$Zip,
    [switch]$Installer
)

$ErrorActionPreference = "Stop"

# ---------- 可配置项 ----------
$AppName   = "GuessingGame"        # 输出目录 / 可执行文件名
$PkgName   = "guessing-game"       # Cargo 包名
$Arch      = "x64"

# ---------- 路径 ----------
$Root   = Split-Path -Parent $PSScriptRoot

# 版本号直接取 Cargo.toml，改版本只需改一处
$Version = (Select-String -Path (Join-Path $Root "project\$PkgName\Cargo.toml") `
            -Pattern '^version = "(.*)"').Matches.Groups[1].Value
$Dist   = Join-Path $Root "dist"
$OutDir = Join-Path $Dist $AppName
$Exe    = Join-Path $Root "target\release\$PkgName.exe"

Set-Location $Root

Write-Host "==> 构建 release 版本"
cargo build --release -p $PkgName
if ($LASTEXITCODE -ne 0) {
    throw "cargo build 失败"
}

if (-not (Test-Path $Exe)) {
    throw "没有找到可执行文件：$Exe"
}

Write-Host "==> 组装输出目录"
if (Test-Path $OutDir) {
    Remove-Item $OutDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Copy-Item $Exe (Join-Path $OutDir "$AppName.exe")

# 中文字体通过 include_bytes! 嵌入 exe；
# 图标和版本信息由 build.rs + winres 在构建时写入 exe 资源段，均无需附带额外文件

if ($Zip) {
    Write-Host "==> 生成压缩包"
    $ZipPath = Join-Path $Dist "$AppName-$Version-windows-$Arch.zip"
    if (Test-Path $ZipPath) {
        Remove-Item $ZipPath -Force
    }
    Compress-Archive -Path (Join-Path $OutDir "*") -DestinationPath $ZipPath
    Write-Host "    $ZipPath"
}

if ($Installer) {
    Write-Host "==> 生成 setup.exe 安装向导"

    # 依次尝试：PATH 中的 ISCC -> 默认安装目录（x86 / x64）
    $Iscc = (Get-Command "ISCC.exe" -ErrorAction SilentlyContinue).Source
    if (-not $Iscc) {
        $candidates = @(
            "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
            "${env:ProgramFiles(x86)}\Inno Setup 7\ISCC.exe",
            "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
            "$env:ProgramFiles\Inno Setup 7\ISCC.exe"
        )
        $Iscc = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    }
    if (-not $Iscc) {
        throw "没有找到 ISCC.exe，请先安装 Inno Setup 6：winget install JRSoftware.InnoSetup"
    }

    $Iss = Join-Path $Root "scripts\windows-installer.iss"
    & $Iscc "/DAppVersion=$Version" "/DSourceExe=$(Join-Path $OutDir "$AppName.exe")" "/DOutputDir=$Dist" $Iss
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup 编译失败"
    }
    Write-Host "    $(Join-Path $Dist "$AppName-$Version-setup.exe")"
}

Write-Host "==> 完成"
Write-Host "    $(Join-Path $OutDir "$AppName.exe")"
