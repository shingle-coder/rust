#!/usr/bin/env bash
# =============================================================================
# 一键打包 macOS .app
#
# 用法：
#   ./scripts/package-macos.sh          # 只生成 .app
#   ./scripts/package-macos.sh --dmg    # 额外生成 .dmg 安装镜像
#
# 做的事：release 构建 -> 组装 .app 目录 -> 写 Info.plist -> ad-hoc 签名 -> 校验
# =============================================================================
set -euo pipefail

# ---------- 可配置项 ----------
APP_NAME="妞妞猜数字"                    # .app / .dmg 文件名（即 Finder 里看到的名字）
DISPLAY_NAME="妞妞猜数字"                # 菜单栏与卷标显示的名称
BUNDLE_ID="com.tanzhenxing.guessing-game"
MIN_MACOS="11.0"
PKG_NAME="guessing-game"                # Cargo 包名
BIN_NAME="guessing-game"                # 可执行文件名
ICON_NAME="AppIcon"                     # Resources 下的 icns 文件名（不含扩展名）

# ---------- 路径 ----------
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
APP="$DIST/$APP_NAME.app"
BINARY="$ROOT/target/release/$BIN_NAME"
ICNS="$ROOT/project/$PKG_NAME/assets/icon.icns"

# 版本号直接取 Cargo.toml，改版本只需改一处
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/project/$PKG_NAME/Cargo.toml" | head -1)"

echo "==> 构建 release 版本"
cd "$ROOT"
cargo build --release -p "$PKG_NAME"

if [[ ! -x "$BINARY" ]]; then
    echo "错误：没有找到可执行文件 $BINARY" >&2
    exit 1
fi

echo "==> 组装 $APP_NAME.app (v$VERSION)"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BINARY" "$APP/Contents/MacOS/$BIN_NAME"
chmod +x "$APP/Contents/MacOS/$BIN_NAME"
printf 'APPL????' > "$APP/Contents/PkgInfo"

if [[ ! -f "$ICNS" ]]; then
    echo "错误：没有找到图标 $ICNS" >&2
    exit 1
fi
cp "$ICNS" "$APP/Contents/Resources/$ICON_NAME.icns"

# Info.plist：CFBundleExecutable 必须与 MacOS/ 下的文件名一致
cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>zh_CN</string>
    <key>CFBundleExecutable</key>
    <string>$BIN_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$DISPLAY_NAME</string>
    <key>CFBundleIconFile</key>
    <string>$ICON_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>$MIN_MACOS</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

echo "==> ad-hoc 签名"
codesign --force --deep --sign - "$APP"

echo "==> 校验"
plutil -lint "$APP/Contents/Info.plist"
codesign --verify --verbose=2 "$APP"

# ---------- 可选：生成 dmg ----------
if [[ "${1:-}" == "--dmg" ]]; then
    DMG="$DIST/$APP_NAME.dmg"
    echo "==> 生成 $APP_NAME.dmg"
    rm -f "$DMG"

    # 必须用一个暂存目录作为镜像根：
    # 直接把 .app 目录传给 srcfolder 会把 .app 的内部文件摊到根目录，
    # 导致挂载后看不到 GuessingGame.app
    STAGE="$DIST/.dmg-stage"
    rm -rf "$STAGE"
    mkdir -p "$STAGE"
    cp -R "$APP" "$STAGE/"
    ln -s /Applications "$STAGE/Applications"   # 拖拽安装用的快捷入口

    # 新版 macOS 用 diskutil，旧版没有该子命令时回退到 hdiutil
    if ! diskutil image create from --format UDZO --volumeName "$DISPLAY_NAME" "$STAGE" "$DMG"; then
        rm -f "$DMG"
        hdiutil create -volname "$DISPLAY_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
    fi
    rm -rf "$STAGE"
    echo "    $DMG"
fi

echo "==> 完成"
echo "    $APP ($(du -sh "$APP" | cut -f1))"
