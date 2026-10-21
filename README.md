# 妞妞猜数字

[![Release](https://img.shields.io/github/v/release/shingle-coder/rust?label=release)](https://github.com/shingle-coder/rust/releases)
[![Build](https://github.com/shingle-coder/rust/actions/workflows/release.yml/badge.svg)](https://github.com/shingle-coder/rust/actions/workflows/release.yml)

用 Rust + [egui](https://github.com/emilk/egui) 写的儿童猜数字小游戏，同时作为 Rust 学习项目的实践案例。

游戏在 1 ~ 100 之间随机藏一个数字，玩家不断猜，直到猜中为止。界面针对 10 岁以下小朋友设计：大字号、高对比配色、圆角胶囊按钮、对话式提示，并配有音效与动画。

## 功能特性

- **猜数字玩法**：每次猜错自动缩窄区间，实时显示「已排除 / 还剩」的概率
- **输入限制**：只能输入 1 ~ 100 的整数，自动过滤非数字字符，越界自动回退
- **玩家系统**：支持多玩家档案的添加、改名、删除与切换
- **成绩记录**：每位玩家保存最好成绩、完成局数、最近 20 局记录
- **排行榜**：按最好成绩排序，当前玩家高亮显示
- **动画效果**：进度条平滑过渡、文字弹跳、猜中彩纸粒子
- **音效**：正弦波实时合成，猜中上行琶音、猜错轻提示，无外部音频文件
- **本地存档**：玩家数据以 JSON 保存，删除文件即可恢复初始状态
- **三端发布**：macOS（.app / .dmg）、Windows（zip / setup.exe）、Linux（tar.gz）
- **SVG 图标**：界面图标使用内置 SVG 矢量图，任意缩放不失真

## 技术栈

| 方向 | 使用 |
| --- | --- |
| 语言 | Rust 2024 edition |
| GUI | eframe / egui 0.37 |
| 随机数 | rand 0.10 |
| 音频 | rodio 0.23（实时合成正弦波） |
| 序列化 | serde + serde_json |
| 图标 | egui_extras（resvg SVG 加载） |
| 打包 | 平台脚本 + Inno Setup + GitHub Actions |

## 目录结构

```
.
├── project/
│   └── guessing-game/          # 猜数字游戏主项目
│       ├── assets/             # 图标、字体、应用图标
│       │   └── icons/          # 界面用 SVG 图标
│       ├── src/
│       │   ├── main.rs         # 界面与游戏逻辑
│       │   ├── tests.rs        # 单元测试（输入清洗 / 配色对比度 / 图标）
│       │   ├── audio.rs        # 音效合成
│       │   └── storage.rs      # 玩家档案与本地存档
│       └── build.rs            # Windows exe 图标与版本信息
├── scripts/                    # 三平台打包脚本
│   ├── package-macos.sh
│   ├── package-windows.ps1
│   ├── windows-installer.iss
│   └── package-linux.sh
├── .github/workflows/          # 跨平台发布工作流
├── book/ · docs/ · rustlings/  # Rust 学习资料
└── Cargo.toml                  # workspace 配置
```

## 本地运行

需要 Rust 工具链（stable）。项目是 Cargo workspace，在仓库根目录执行：

```bash
cargo run -p guessing-game
```

发布构建：

```bash
cargo build -p guessing-game --release
```

## 单元测试

```bash
cargo test -p guessing-game
```

测试覆盖输入清洗（`sanitize_guess`）、玩家数据（档案记录、排行榜、名字校验），以及界面相关的卡片与背景文字对比度、按钮文字对比度、卡片描边、渐变网格、背景闪光顶点和全部内置 SVG 图标的解码结果；另有 egui 0.37 的无窗口整帧渲染与 rand 0.10 的随机答案范围检查。

## 打包

各平台脚本会自动从 `Cargo.toml` 读取版本号，构建产物输出到根目录的 `dist/`。

**macOS**（在 macOS 上执行）：

```bash
./scripts/package-macos.sh         # 生成 .app
./scripts/package-macos.sh --dmg   # 额外生成 .dmg
```

**Windows**（在 Windows + PowerShell 上执行）：

```powershell
./scripts/package-windows.ps1 -Zip              # 生成 zip
./scripts/package-windows.ps1 -Installer        # 额外生成安装向导（需要 Inno Setup）
```

**Linux**（在 Linux 上执行）：

```bash
./scripts/package-linux.sh              # 生成 tar.gz
./scripts/package-linux.sh --appimage   # 额外生成 AppImage（需要 appimagetool）
```

Linux 构建前置依赖：`build-essential`、`pkg-config`、`libasound2-dev` 以及 X11 / Wayland 相关开发库。

## 自动发布

推送 `v*` 标签即可触发三端构建并自动创建 GitHub Release：

```bash
git tag v0.1.3
git push origin v0.1.3
```

工作流定义见 [.github/workflows/release.yml](.github/workflows/release.yml)。

## 存档位置

| 平台 | 路径 |
| --- | --- |
| macOS | `~/Library/Application Support/NiuniuGuess/players.json` |
| Windows | `%APPDATA%\NiuniuGuess\players.json` |
| Linux | `~/.local/share/NiuniuGuess/players.json` |

## 图标说明

界面图标位于 `project/guessing-game/assets/icons/`，采用 24×24 彩色填充的卡通风格 SVG，通过 `egui_extras` 的 `svg` 特性在运行时解码并按需缩放，不再做单色 tint 处理。图标以 `include_image!` 方式编译进二进制，发布后无需额外资源文件。

界面整体是奶油色渐变背景 + 彩色圆角卡片的绘本风，文字统一为暖棕色，按钮为糖果色胶囊样式。

应用图标（`icon.icns` / `icon.ico` / `icon.png`）是同一份卡通兔子插画导出的多尺寸资源，三端打包脚本会自动取用。

游戏主体使用捆绑的苹方字体（`assets/fonts/PingFang-Regular.ttf`）保证中文显示，同样通过 `include_bytes!` 内嵌。
