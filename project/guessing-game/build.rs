// Windows 资源编译：给 exe 嵌入图标 + 版本信息。
// macOS / Linux 上这个 build script 是空操作，不影响正常构建。

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();

    // 资源管理器里显示的 exe 图标（多尺寸 ICO）
    res.set_icon("assets/icon.ico");

    // 版本号从 Cargo.toml 的 package.version 读取，避免两处维护
    let parts: Vec<u64> = env!("CARGO_PKG_VERSION")
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    let ver = (parts.first().copied().unwrap_or(0) << 48)
        | (parts.get(1).copied().unwrap_or(0) << 32)
        | (parts.get(2).copied().unwrap_or(0) << 16);

    res.set_version_info(winres::VersionInfo::FILEVERSION, ver);
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, ver);

    // 属性 -> 详细信息 里显示的字段
    res.set("ProductName", "妞妞猜数字");
    res.set("FileDescription", "妞妞猜数字");
    res.set("OriginalFilename", "GuessingGame.exe");
    res.set("LegalCopyright", "");

    res.compile().expect("编译 Windows 资源失败");
}

#[cfg(not(windows))]
fn main() {}
