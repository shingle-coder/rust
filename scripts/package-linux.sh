#!/usr/bin/env bash
# =============================================================================
# Linux 打包脚本：生成便携 tar.gz，可选生成 AppImage
#
# 用法（在 Linux 上、仓库根目录执行）：
#   ./scripts/package-linux.sh             # dist/GuessingGame-0.1.0-linux-x86_64.tar.gz
#   ./scripts/package-linux.sh --appimage   # 额外生成 .AppImage（需 appimagetool）
#
# AppImage 依赖：appimagetool（PATH 中，或放在 tools/appimagetool）
#   下载：https://github.com/AppImage/appimagetool/releases
#
# 注意：egui/eframe 的构建依赖 X11/Wayland 开发库，Ubuntu 上通常需要：
#   sudo apt install build-essential pkg-config libx11-dev libxcb1-dev \
#        libxkbcommon-dev libwayland-dev libgl1-mesa-dev
# =============================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT="$ROOT/project/guessing-game"
BIN_NAME="guessing-game"
APP_NAME="GuessingGame"
DISPLAY_NAME="妞妞猜数字"
DIST="$ROOT/dist"

VERSION="$(grep -m1 '^version' "$PROJECT/Cargo.toml" | sed -E 's/.*"(.*)".*/\1/')"
if [[ -z "$VERSION" ]]; then
    echo "无法从 Cargo.toml 读取版本号" >&2
    exit 1
fi

case "$(uname -m)" in
    x86_64)  ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *)       ARCH="$(uname -m)" ;;
esac

echo "==> 构建 release 版本 ($APP_NAME $VERSION, $ARCH)"
cd "$ROOT"
cargo build --release -p guessing-game

mkdir -p "$DIST"
PORTABLE="$DIST/$APP_NAME"
rm -rf "$PORTABLE"
mkdir -p "$PORTABLE"

# ---------- 便携目录：可执行文件 + 图标 + 桌面入口 ----------
cp "$ROOT/target/release/$BIN_NAME" "$PORTABLE/$APP_NAME"
chmod +x "$PORTABLE/$APP_NAME"
cp "$PROJECT/assets/icon-256.png" "$PORTABLE/$APP_NAME.png"

cat > "$PORTABLE/$APP_NAME.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=$DISPLAY_NAME
Name[zh_CN]=$DISPLAY_NAME
Comment=妞妞猜数字小游戏
Exec=$APP_NAME
Icon=$APP_NAME
Terminal=false
Categories=Game;PuzzleGame;
EOF

TARBALL="$DIST/$APP_NAME-$VERSION-linux-$ARCH.tar.gz"
rm -f "$TARBALL"
tar -C "$DIST" -czf "$TARBALL" "$APP_NAME"
echo "    $TARBALL"

# ---------- 可选：AppImage ----------
if [[ "${1:-}" == "--appimage" ]]; then
    TOOL="$(command -v appimagetool || true)"
    if [[ -z "$TOOL" && -x "$ROOT/tools/appimagetool" ]]; then
        TOOL="$ROOT/tools/appimagetool"
    fi
    if [[ -z "$TOOL" ]]; then
        echo "错误：找不到 appimagetool，见脚本头部下载地址" >&2
        exit 1
    fi

    echo "==> 生成 AppImage"
    APPDIR="$DIST/$APP_NAME.AppDir"
    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin" \
             "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
             "$APPDIR/usr/share/applications"

    cp "$ROOT/target/release/$BIN_NAME" "$APPDIR/usr/bin/$BIN_NAME"
    chmod +x "$APPDIR/usr/bin/$BIN_NAME"
    cp "$PROJECT/assets/icon-256.png" \
       "$APPDIR/usr/share/icons/hicolor/256x256/apps/$BIN_NAME.png"
    cp "$PORTABLE/$APP_NAME.desktop" "$APPDIR/$APP_NAME.desktop"
    cp "$PROJECT/assets/icon-256.png" "$APPDIR/$APP_NAME.png"
    ln -sf "usr/bin/$BIN_NAME" "$APPDIR/AppRun"

    APPIMAGE="$DIST/$APP_NAME-$VERSION-linux-$ARCH.AppImage"
    rm -f "$APPIMAGE"
    ARCH="$ARCH" "$TOOL" "$APPDIR" "$APPIMAGE"
    rm -rf "$APPDIR"
    echo "    $APPIMAGE"
fi

echo "==> 完成"
echo "    $PORTABLE"
