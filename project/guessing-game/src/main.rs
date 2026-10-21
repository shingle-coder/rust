// =============================================================================
// 猜数字游戏 · Guessing Game
// -----------------------------------------------------------------------------
// 功能：
//   - egui 即时模式 GUI 的猜数字游戏
//   - 1~100 范围随机数，玩家反复猜测直到猜对
//   - 每次猜错自动缩窄提示区间（二分思路）
//   - 左侧栏显示统计 / 主区展示反馈 / 可折叠历史记录
//   - 中文字体优先加载 macOS 苹方（PingFang）
//
// 主要 Rust 知识点：
//   结构体 + enum 管理游戏状态
//   Default trait 提供初始值
//   rand 0.9 的 random_range / Rng trait
//   egui / eframe 即时模式 GUI
// =============================================================================

// Windows release 构建时隐藏控制台窗口：默认是 console 子系统，双击 exe 会先弹黑框。
// 只在非 debug 构建生效，cargo run 时仍保留控制台方便看 println 日志。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use rand::Rng;

// 游戏主状态结构体，eframe::App 的每帧 update(&mut self) 都在修改它
struct GuessingGame {
    /// 当前要猜的目标数字（游戏开始时随机生成）
    secret: u32,
    /// 输入框的内容，每次尝试后清空
    guess: String,
    /// 已猜测次数，猜对时用于展示
    attempts: u32,
    /// 历史记录列表，用于可折叠面板展示
    history: Vec<HistoryItem>,
    /// 最近一次的反馈文案（成功/太小/太大/非法输入）
    last_hint: String,
    /// 经过猜测后推断出的下界（不可能比它小）
    min_bound: u32,
    /// 经过猜测后推断出的上界（不可能比它大）
    max_bound: u32,
    /// 是否已经猜对（控制胜利画面与输入画面的切换）
    won: bool,
}

// 单条历史记录
#[derive(Clone)]
struct HistoryItem {
    n: u32,              // 猜的数字
    hint: String,        // 中文提示，如 "太小了 ⬆"
    kind: HintKind,      // 枚举类型，用于着色
}

// 三种提示类型，决定历史徽章的颜色
#[derive(Clone, Copy, PartialEq, Eq)]
enum HintKind {
    TooSmall,
    TooBig,
    Correct,
}

// Default trait：Rust 中标准的"给一个合理初值"方式
// eframe::run_native 需要 App 的 Box，这里 Default::default() 会调 new_game()
impl Default for GuessingGame {
    fn default() -> Self {
        Self::new_game()
    }
}

impl GuessingGame {
    // 游戏范围常量：const 编译期确定，效率高
    const MIN: u32 = 1;
    const MAX: u32 = 100;

    /// 创建一局新游戏，所有状态重置，随机生成 secret
    fn new_game() -> Self {
        Self {
            // rand 0.9 的 random_range 是 RangeInclusive（..=）
            secret: rand::rng().random_range(Self::MIN..=Self::MAX),
            guess: String::new(),
            attempts: 0,
            history: Vec::new(),
            last_hint: "输入 1~100 的整数，按回车或点「猜！」".into(),
            min_bound: Self::MIN,
            max_bound: Self::MAX,
            won: false,
        }
    }

    /// 核心猜测逻辑：解析输入、更新状态、给出提示
    /// 用 match 处理 parse 的 Result，再用 Ordering 处理 cmp 的结果
    fn try_guess(&mut self) {
        let trimmed = self.guess.trim();

        // 先做合法性校验
        match trimmed.parse::<u32>() {
            Ok(n) if n < Self::MIN || n > Self::MAX => {
                self.last_hint = format!("⚠ 请输入 {}~{} 之间的整数", Self::MIN, Self::MAX);
            }

            Ok(n) => {
                self.attempts += 1;

                // n.cmp(&secret) 返回 Ordering::Less/Greater/Equal
                // match 同时处理缩窄区间、返回提示文本和 HintKind
                let (hint, kind) = match n.cmp(&self.secret) {
                    std::cmp::Ordering::Less => {
                        // 猜小了 → 下界抬高为 n+1（secret 不可能 ≤ n）
                        self.min_bound = self.min_bound.max(n + 1);
                        ("太小了 ⬆", HintKind::TooSmall)
                    }
                    std::cmp::Ordering::Greater => {
                        // 猜大了 → 上界压低为 n-1
                        self.max_bound = self.max_bound.min(n - 1);
                        ("太大了 ⬇", HintKind::TooBig)
                    }
                    std::cmp::Ordering::Equal => {
                        self.won = true;
                        (
                            &format!(
                                "🎉 猜对了！答案就是 {}，总共猜了 {} 次",
                                self.secret, self.attempts
                            ) as &str,
                            HintKind::Correct,
                        )
                    }
                };

                self.last_hint = hint.to_string();
                self.history.push(HistoryItem {
                    n,
                    hint: hint.to_string(),
                    kind,
                });
            }

            Err(_) => {
                self.last_hint = "⚠ 请输入有效的整数".into();
            }
        }

        // 无论成功/失败都清空输入框，方便下一次
        self.guess.clear();
    }

    /// 根据 hint 文案返回对应颜色（UI 侧用）
    /// 颜色含义：绿=赢，橙=非法输入，蓝=太小，红=太大
    fn hint_color(hint: &str) -> egui::Color32 {
        if hint.contains("猜对") || hint.contains("🎉") {
            egui::Color32::from_rgb(80, 220, 120)
        } else if hint.starts_with("⚠") {
            egui::Color32::from_rgb(255, 170, 80)
        } else if hint.contains("太小") {
            egui::Color32::from_rgb(120, 180, 255)
        } else if hint.contains("太大") {
            egui::Color32::from_rgb(255, 130, 130)
        } else {
            egui::Color32::from_rgb(210, 210, 220)
        }
    }
}

// range_bar 需要访问 MIN/MAX，但它不是 GuessingGame 的方法
// Rust 不允许自由函数访问 struct 的关联 const，用两个 free const 镜像一下
const RANGE_MIN: u32 = GuessingGame::MIN;
const RANGE_MAX: u32 = GuessingGame::MAX;

/// 在 egui UI 上画一个"当前可猜区间"可视化条
/// 灰色轨 + 蓝色高亮区间，两端标注 min/max 数字
fn range_bar(ui: &mut egui::Ui, min: u32, max: u32) {
    // 计算可猜范围占总范围的比例
    let total = (RANGE_MAX - RANGE_MIN + 1) as f32;
    let lo_pct = (min - RANGE_MIN) as f32 / total;
    let hi_pct = (max - RANGE_MIN + 1) as f32 / total;

    let available = ui.available_width();
    let bar_height = 14.0;

    // 手动分配一块区域并画上，不依赖 egui 的现成组件
    let response = ui.allocate_response(
        egui::vec2(available, bar_height + 14.0),
        egui::Sense::hover(),
    );
    let rect = response.rect;

    // 先画灰色轨道
    let track_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(0.0, 10.0),
        egui::vec2(available, bar_height),
    );
    ui.painter().rect_filled(
        track_rect,
        egui::CornerRadius::same(7),
        egui::Color32::from_rgb(55, 58, 72),
    );

    // 再画蓝色高亮段（当前剩余可猜范围）
    let filled_rect = egui::Rect::from_min_size(
        track_rect.min + egui::vec2(available * lo_pct, 0.0),
        egui::vec2(available * (hi_pct - lo_pct), bar_height),
    );
    ui.painter().rect_filled(
        filled_rect,
        egui::CornerRadius::same(6),
        egui::Color32::from_rgb(100, 160, 255),
    );

    // 左端点标注 min
    ui.painter().text(
        rect.left_top() + egui::vec2(0.0, -2.0),
        egui::Align2::LEFT_TOP,
        format!("{}", min),
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(180, 200, 255),
    );

    // 右端点标注 max
    ui.painter().text(
        rect.right_top() + egui::vec2(0.0, -2.0),
        egui::Align2::RIGHT_TOP,
        format!("{}", max),
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(180, 200, 255),
    );
}

// =============================================================================
// eframe::App 实现 —— 每帧被 egui 调用一次 update
// =============================================================================

impl eframe::App for GuessingGame {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---------- 左侧面板：统计信息 ----------
        egui::SidePanel::left("stats_panel")
            .resizable(true)
            .default_width(200.0)
            .width_range(170.0..=260.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.heading(
                    egui::RichText::new("📊 战况")
                        .size(18.0)
                        .color(egui::Color32::from_rgb(240, 240, 255)),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("{} ~ {} · 二分提示", Self::MIN, Self::MAX))
                        .small()
                        .color(egui::Color32::from_rgb(150, 155, 175)),
                );
                ui.add_space(12.0);

                // 统计卡片：范围 + 已猜次数
                egui::Frame::group(ui.style())
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("范围")
                                .small()
                                .color(egui::Color32::from_rgb(160, 160, 180)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{} ~ {}", Self::MIN, Self::MAX))
                                .size(16.0)
                                .strong(),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("已猜次数")
                                .small()
                                .color(egui::Color32::from_rgb(160, 160, 180)),
                        );
                        ui.label(
                            egui::RichText::new(self.attempts.to_string())
                                .size(22.0)
                                .color(egui::Color32::from_rgb(255, 200, 120))
                                .strong(),
                        );
                    });

                ui.add_space(8.0);

                // 提示区间卡片：始终显示，避免猜对后左侧布局突然跳变
                egui::Frame::group(ui.style())
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("提示区间")
                                .small()
                                .color(egui::Color32::from_rgb(160, 160, 180)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "[{}, {}]",
                                self.min_bound, self.max_bound
                            ))
                            .size(16.0)
                            .color(egui::Color32::from_rgb(120, 180, 255))
                            .strong(),
                        );
                        ui.add_space(4.0);
                        // 剩余区间占总范围的比例（最小 1，避免除零）
                        let span = (self.max_bound - self.min_bound + 1).max(1);
                        let ratio = span as f32 / (Self::MAX - Self::MIN + 1) as f32;
                        ui.label(
                            egui::RichText::new(format!("剩余范围 {:.0}%", ratio * 100.0))
                                .small()
                                .color(egui::Color32::from_rgb(130, 160, 200)),
                        );
                        ui.add(
                            egui::ProgressBar::new(1.0 - ratio)
                                .desired_width(ui.available_width())
                                .text(format!("已排除 {:.0}%", (1.0 - ratio) * 100.0))
                                .fill(egui::Color32::from_rgb(100, 160, 255)),
                        );
                    });

                // 底部操作提示（猜对后不再需要）
                if !self.won {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("回车提交 · 或点击「🎲 猜！」")
                            .small()
                            .color(egui::Color32::from_rgb(120, 125, 145)),
                    );
                }
            });

        // ---------- 中央区域：主游戏界面 ----------
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::group(ui.style())
                .corner_radius(egui::CornerRadius::same(14))
                .inner_margin(egui::Margin::symmetric(20, 16))
                .show(ui, |ui| {
                    // 顶部：左标题 + 右副标题
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("🎯 猜数字游戏")
                                .size(22.0)
                                .color(egui::Color32::from_rgb(240, 240, 255)),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "目标：{} ~ {}",
                                        Self::MIN, Self::MAX
                                    ))
                                    .small()
                                    .color(egui::Color32::from_rgb(160, 160, 180)),
                                );
                            },
                        );
                    });

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("当前可猜区间")
                            .small()
                            .color(egui::Color32::from_rgb(160, 160, 180)),
                    );
                    range_bar(ui, self.min_bound, self.max_bound);
                    ui.add_space(8.0);

                    // 深色反馈卡片：猜对时背景变绿色
                    egui::Frame::group(ui.style())
                        .corner_radius(egui::CornerRadius::same(12))
                        .inner_margin(egui::Margin::symmetric(16, 14))
                        .fill(if self.won {
                            egui::Color32::from_rgb(40, 80, 55)
                        } else {
                            egui::Color32::from_rgb(35, 38, 50)
                        })
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                if self.won {
                                    // ===== 胜利画面 =====
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "🎉  答案是 {}  🎉",
                                            self.secret
                                        ))
                                        .size(26.0)
                                        .color(egui::Color32::from_rgb(120, 240, 140))
                                        .strong(),
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "总共猜了 {} 次",
                                            self.attempts
                                        ))
                                        .color(egui::Color32::from_rgb(200, 255, 210)),
                                    );
                                    ui.add_space(10.0);
                                    if ui
                                        .add_sized(
                                            [180.0, 42.0],
                                            egui::Button::new(
                                                egui::RichText::new("🔄 再来一局").size(18.0),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        *self = Self::new_game();
                                    }
                                } else {
                                    // ===== 游戏中画面 =====
                                    let color = Self::hint_color(&self.last_hint);
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(&self.last_hint)
                                            .size(18.0)
                                            .color(color)
                                            .strong(),
                                    );
                                    ui.add_space(14.0);

                                    // 输入框 + 猜按钮（回车也能触发）
                                    ui.horizontal(|ui| {
                                        let resp = ui.add_sized(
                                            [160.0, 38.0],
                                            egui::TextEdit::singleline(&mut self.guess)
                                                .hint_text("输入数字…")
                                                .font(egui::TextStyle::Monospace),
                                        );
                                        let guess_btn = ui.add_sized(
                                            [110.0, 38.0],
                                            egui::Button::new(
                                                egui::RichText::new("🎲 猜！").size(16.0),
                                            ),
                                        );

                                        // 回车触发：lost_focus + Key::Enter
                                        if resp.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            self.try_guess();
                                        }
                                        if guess_btn.clicked() {
                                            self.try_guess();
                                        }
                                    });
                                }
                            });
                        });

                    // 历史记录面板（可折叠，超过 200px 滚动）
                    if !self.history.is_empty() {
                        ui.add_space(12.0);
                        ui.collapsing(
                            egui::RichText::new(format!("📜 历史记录 ({})", self.history.len())),
                            |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        for (i, item) in self.history.iter().enumerate() {
                                            let badge = match item.kind {
                                                HintKind::TooSmall => {
                                                    egui::Color32::from_rgb(60, 100, 160)
                                                }
                                                HintKind::TooBig => {
                                                    egui::Color32::from_rgb(180, 80, 80)
                                                }
                                                HintKind::Correct => {
                                                    egui::Color32::from_rgb(60, 140, 90)
                                                }
                                            };
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("#{}", i + 1))
                                                        .small()
                                                        .color(egui::Color32::from_rgb(140, 140, 160))
                                                        .monospace(),
                                                );
                                                ui.add_space(2.0);
                                                egui::Frame::group(ui.style())
                                                    .corner_radius(egui::CornerRadius::same(4))
                                                    .fill(badge)
                                                    .inner_margin(egui::Margin::symmetric(8, 2))
                                                    .show(ui, |ui| {
                                                        ui.label(
                                                            egui::RichText::new(format!("{}", item.n))
                                                                .strong(),
                                                        );
                                                    });
                                                ui.add_space(6.0);
                                                ui.label(
                                                    egui::RichText::new(&item.hint)
                                                        .color(Self::hint_color(&item.hint)),
                                                );
                                            });
                                            ui.add_space(3.0);
                                        }
                                    });
                            },
                        );
                    }
                });
        });
    }
}

// =============================================================================
// 中文字体加载
// =============================================================================
// egui 默认打包的字体只有拉丁字形，中文会显示为空白/方块 □，
// 所以必须额外提供一个带 CJK 的字体。
//
// 官方文档（epaint 0.33.3 FontData）：
//   FontData { font: Cow<'static, [u8]>, index: u32, tweak: FontTweak }
//   - font  ：字体文件字节
//   - index ：TTC 集合里的子字体索引，单文件 TTF 用 0 即可
// 用法即：FontDefinitions::default() → font_data.insert → families 里插到最前面
//        → ctx.set_fonts(fonts)
//
// 字体文件通过 include_bytes! 在编译期嵌入二进制：
//   - 发布后把可执行文件单独拷给别人也能正常显示中文
//   - 不依赖源码目录（CARGO_MANIFEST_DIR 只是编译机上的路径，运行时可能不存在）
//   - 不需要把 assets 目录一起分发；代价是二进制体积增加约 10MB
//
// 重要坑：不要使用系统私有目录里的 PingFangUI.ttc
//   /System/Library/PrivateFrameworks/.../Reserved/PingFangUI.ttc
//   它是 macOS 内部保留字体，ab_glyph 能解析出 glyph id，但取不到字形轮廓，
//   结果不是"中文乱码"，而是整个窗口所有文字（含英文数字）全部不显示。

fn load_chinese_font(ctx: &egui::Context) {
    // 编译期读取项目内的苹方字体，直接进入只读数据段（零运行时 I/O）
    const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/PingFang-Regular.ttf");

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "zh".into(),
        egui::FontData {
            // Cow::Borrowed，不复制这 10MB 数据
            font: FONT_BYTES.into(),
            index: 0,
            tweak: Default::default(),
        }
        .into(),
    );

    // 插到字体族列表最前面 → 优先用中文字体渲染；
    // 中文没有的字形（Emoji 等）会自动 fallback 到列表里后面的 egui 默认字体
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "zh".into());
    }

    ctx.set_fonts(fonts);
}

// =============================================================================
// 入口：配置窗口 + 加载字体 + 启动主循环
// =============================================================================

fn main() -> eframe::Result<()> {
    // 运行时窗口图标（macOS 的 Dock 悬停 / Windows 任务栏）：
    // 和 .icns、.ico 用的是同一张图，这里嵌一张 256px 的 PNG，避免运行时读文件
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png"))
        .expect("内置窗口图标解析失败");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([680.0, 560.0])       // 初始窗口大小
            .with_min_inner_size([520.0, 440.0])  // 最小可缩大小
            .with_title("猜数字 · Guessing Game")
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "guessing-game",
        options,
        Box::new(|cc| {
            // cc = CreationContext，在 App 构造前只调一次
            // 这里加载中文字体，之后 App 的每帧 update 就都能渲染中文了
            load_chinese_font(&cc.egui_ctx);

            // 界面里的文字/配色都是按深色底设计的，固定深色主题；
            // 否则跟随系统浅色主题时，接近白色的标题会几乎看不见
            cc.egui_ctx.set_theme(egui::ThemePreference::Dark);

            Ok(Box::<GuessingGame>::default())
        }),
    )
}
