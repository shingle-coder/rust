# Rust 环境搭建 FAQ

本文档记录本机（macOS aarch64）Rust 工具链安装与排障过程。

---

## Q1: `rustup` 报 `.profile` Permission denied

### 现象

```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
...
error: could not amend shell profile: '/Users/tanzhenxing/.profile':
  could not write rcfile file: '/Users/tanzhenxing/.profile': Permission denied (os error 13)
```

### 根因

`.profile` 文件属主是 root：

```
-rw-r--r--  1 root  staff  148  4月  7  2023 /Users/tanzhenxing/.profile
```

rustup 安装程序尝试向 `.profile` 写入 PATH 行时无权限。

### 处理

**工具链本体其实已经装好了**，PATH 写入失败只是 shell 配置没更新。手动加 PATH 即可：

```bash
# 临时（当前会话生效）
export PATH="$HOME/.cargo/bin:$PATH"

# 永久（macOS 默认 zsh，推荐写入 .zshrc 或 .zshenv）
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

也可以先修 `.profile` 权限再重跑：

```bash
sudo chown $(whoami) ~/.profile
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Q2: 系统提示已有 `settings.toml`，但 `rustup` / `rustc` 找不到

### 现象

```
$ rustup
zsh: command not found: rustup
```

但手动调用安装脚本时看到：

```
warn: it looks like you have an existing rustup settings file at:
warn: /Users/tanzhenxing/.rustup/settings.toml
```

### 根因

两种可能：

1. **历史安装残留**——`.rustup/` 目录存在但 `.cargo/bin` 没加进 PATH
2. **工具链损坏**——之前 rustup update 或安装被中断，toolchain manifest 丢失

### 诊断与修复

```bash
# 1. 临时加 PATH 让 rustup 可用
export PATH="$HOME/.cargo/bin:$PATH"

# 2. 查看当前状态
rustup show        # 显示已安装工具链
rustc --version    # 若报错 "missing manifest" 或 "rustc is not applicable" → 工具链损坏

# 3. 损坏的话彻底重装 stable
rm -rf ~/.rustup/toolchains/stable-aarch64-apple-darwin
rustup toolchain install stable --profile default
```

---

## Q3: rustup update 后报错 "rustc binary is not applicable"

### 现象

```
error: the 'rustc' binary, normally provided by the 'rustc' component,
  is not applicable to the 'stable-aarch64-apple-darwin' toolchain
```

### 根因

rustup 自我更新后（从旧版升到 1.29.1）与原 toolchain manifest 不兼容，toolchain 目录中二进制和 manifest 对不上（实测 rustc 组件缺失，只剩 cargo、clippy-driver）。

### 修复

```bash
export PATH="$HOME/.cargo/bin:$PATH"

# 卸载旧 toolchain 并重新安装
rustup toolchain uninstall stable
rustup toolchain install stable --profile default
```

或者更暴力地删目录重装：

```bash
rm -rf ~/.rustup/toolchains/stable-aarch64-apple-darwin
rustup toolchain install stable --profile default
```

---

## Q4: 如何验证安装成功

```bash
export PATH="$HOME/.cargo/bin:$PATH"

rustc --version     # 应输出类似 rustc 1.99.1 (58cf8f9c3 2026-10-13)
cargo --version     # 应输出类似 cargo 1.99.1
rustup show         # 查看完整工具链状态
rustup update       # 保持工具链最新
```

---

## Q5: rust-analyzer 报 "failed to fetch workspace" / "failed to find any projects"

### 现象

```
ERROR failed to find any projects in [AbsPathBuf("/Users/tanzhenxing/rust")]
ERROR FetchWorkspaceError: rust-analyzer failed to fetch workspace
```

### 根因

rust-analyzer 在**工作区根目录**（Trae/VS Code 打开的那个文件夹）找 `Cargo.toml`。如果你的 Rust 项目嵌在子目录里（比如 `rust/project/hello-world/`），根目录没有 `Cargo.toml`，rust-analyzer 就会报找不到项目。

### 修复

在工作区根目录建一个**虚拟 workspace** `Cargo.toml`：

```toml
# /Users/tanzhenxing/rust/Cargo.toml
[workspace]
members = [
    "project/hello-world",
    # 以后再加项目，往这里追加
]
resolver = "2"
```

然后重启 rust-analyzer：

- 命令面板 → "Restart rust-analyzer server"
- 或者 `Cmd+Shift+P` → "Reload Window"

### 验证

```bash
cd /Users/tanzhenxing/rust
cargo check --workspace   # 编译通过就说明 workspace 正确
```

---

## Q6: 子目录里多余的 `Cargo.lock`

### 现象

workspace 建好后，根目录和子 package 目录各有一份 `Cargo.lock`。

### 规则

Cargo 一旦进入 workspace 模式：

| 文件 | 位置 | 状态 |
| --- | --- | --- |
| 根 `Cargo.toml` | workspace 根 | **必须有**，声明 members |
| 子 `Cargo.toml` | package 目录 | **必须有**，package 自身配置 |
| 根 `Cargo.lock` | workspace 根 | **统一锁文件**，所有成员共用 |
| 子 `Cargo.lock` | package 目录 | **被忽略**，可删 |
| `target/` | workspace 根 | **统一编译产物目录** |

### 清理

```bash
rm /Users/tanzhenxing/rust/project/hello-world/Cargo.lock
# 根那份留着就行
```

---

## Q7: GitHub / raw.githubusercontent.com 连不上（国内网络）

### 现象

```
$ curl -L https://raw.githubusercontent.com/rust-lang/rustlings/main/install.sh | bash -s
curl: (28) Failed to connect to raw.githubusercontent.com port 443 after 75008 ms
```

rustup 也可能超时（toolchain 下载走 static.rust-lang.org）。

### 方案 A：装 Rustlings 用 cargo 代替

Rustlings 在 crates.io 上有发布版，不一定要从 GitHub 装脚本：

```bash
cargo install rustlings --locked
cd /path/to/rust-projects
rustlings init
```

### 方案 B：配 cargo 国内镜像（推荐）

在 `~/.cargo/config.toml` 里把 crates.io 替换成 rsproxy：

```toml
[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[registries.rsproxy]
index = "https://rsproxy.cn/crates.io-index"

[net]
git-fetch-with-cli = true
```

### 方案 C：配 rustup 镜像

临时设置环境变量让 rustup 走 ustc：

```bash
export RUSTUP_DIST_SERVER="https://mirrors.ustc.edu.cn/rust-static"
export RUSTUP_UPDATE_ROOT="https://mirrors.ustc.edu.cn/rust-static/rustup"

# 然后执行 rustup update 或 install.sh
```

永久生效的话写进 `.zshrc`：

```bash
echo 'export RUSTUP_DIST_SERVER="https://mirrors.ustc.edu.cn/rust-static"' >> ~/.zshrc
echo 'export RUSTUP_UPDATE_ROOT="https://mirrors.ustc.edu.cn/rust-static/rustup"' >> ~/.zshrc
```

### 国内可用的 Rust 镜像

| 用途 | 镜像 | URL |
| --- | --- | --- |
| crates.io 索引 | rsproxy | https://rsproxy.cn/ |
| rustup 分发 | 中科大 | https://mirrors.ustc.edu.cn/rust-static/ |
| 文档 | 清华 TUNA | https://mirrors.tuna.tsinghua.edu.cn/rust/ |

---

## 参考

- rustup 安装: https://rustup.rs/
- rustup 文档: https://rust-lang.github.io/rustup/
- Shell 配置说明: https://unix.stackexchange.com/questions/38175/difference-between-login-shell-and-non-login-shell

### zsh 配置文件加载顺序

| 文件 | 加载时机 | 说明 |
| --- | --- | --- |
| `~/.zshenv` | 所有 zsh 启动时 | 最通用，适合 PATH |
| `~/.zprofile` | login shell | 等价于 bash 的 `.bash_profile` |
| `~/.zshrc` | interactive shell | 最常用，补全、alias、PATH 都写这里 |
| `~/.zlogin` | login shell 之后 | 登录后执行一次 |
