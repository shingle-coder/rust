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
//   rand 0.10 的 random_range / RngExt trait
//   egui / eframe 即时模式 GUI
// =============================================================================

// Windows release 构建时隐藏控制台窗口：默认是 console 子系统，双击 exe 会先弹黑框。
// 只在非 debug 构建生效，cargo run 时仍保留控制台方便看 println 日志。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod storage;

use audio::Audio;
use eframe::egui;
use rand::RngExt;
use storage::SaveData;

/// 玩家弹窗的两种模式
#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayerDialog {
    Add,
    Rename,
}

// 游戏主状态结构体，eframe::App 的每帧 update(&mut self) 都在修改它
struct GuessingGame {
    /// 当前要猜的目标数字（游戏开始时随机生成）
    secret: u32,
    /// 输入框的内容，每次尝试后清空
    guess: String,
    /// 已猜测次数，猜对时用于展示
    attempts: u32,
    /// 最近一次的反馈文案（成功/太小/太大/非法输入）
    last_hint: String,
    /// 经过猜测后推断出的下界（不可能比它小）
    min_bound: u32,
    /// 经过猜测后推断出的上界（不可能比它大）
    max_bound: u32,
    /// 是否已经猜对（控制胜利画面与输入画面的切换）
    won: bool,
    /// 进度条当前显示的百分比（平滑追赶真实值，产生动画）
    shown_excluded: f32,
    /// 反馈文字 / 次数数字的弹跳强度，提交后从 1 衰减到 0
    pop: f32,
    /// 猜对时撒下的彩纸
    confetti: Vec<Confetti>,
    /// 本帧需要生成彩纸（胜利那一帧置位）
    confetti_pending: bool,
    /// 音效输出（设备打不开时为 None）
    audio: Option<Audio>,
    /// 玩家档案与当前玩家
    store: SaveData,
    /// 正在编辑的玩家弹窗
    player_dialog: Option<PlayerDialog>,
    /// 弹窗里的名字输入
    player_input: String,
    /// 弹窗错误提示
    player_error: Option<String>,
    /// 删除玩家需要点两次确认
    player_delete_confirm: bool,
}

/// 一片飘落的彩纸，坐标用 0~1 的归一化值，绘制时再换算成像素
struct Confetti {
    pos: egui::Pos2,
    vel: egui::Vec2,
    angle: f32,
    spin: f32,
    size: f32,
    color: egui::Color32,
    life: f32,
}

/// 在卡片区域内画出会旋转的彩纸
fn draw_confetti(ui: &egui::Ui, rect: egui::Rect, confetti: &[Confetti]) {
    let painter = ui.painter();
    for c in confetti {
        let center = egui::pos2(
            rect.left() + c.pos.x * rect.width(),
            rect.top() + c.pos.y * rect.height(),
        );
        let (sn, cs) = (c.angle.sin(), c.angle.cos());
        let hx = c.size * 0.5;
        let hy = c.size * 0.3;
        let corner = |dx: f32, dy: f32| center + egui::vec2(dx * cs - dy * sn, dx * sn + dy * cs);
        let points = vec![
            corner(-hx, -hy),
            corner(hx, -hy),
            corner(hx, hy),
            corner(-hx, hy),
        ];
        // 快消失时变透明，避免突然消失
        let color = if c.life < 1.0 {
            c.color.gamma_multiply(c.life)
        } else {
            c.color
        };
        painter.add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::NONE,
        ));
    }
}

// =============================================================================
// 内置 SVG 图标
// -----------------------------------------------------------------------------
// 用 include_bytes! 把图标打进二进制，运行时不需要外部文件。
// SVG 由 egui_extras 的 svg loader（resvg）解码，可按任意尺寸缩放。
// 图标本身就是彩色填充的卡通风格，所以这里不再做 tint 着色。
// =============================================================================

/// 加载一个内置彩色 SVG 图标，按目标尺寸缩放
fn icon(source: egui::ImageSource<'static>, size: f32) -> egui::Image<'static> {
    egui::Image::new(source).fit_to_exact_size(egui::vec2(size, size))
}

// =============================================================================
// 绘本风配色与绘制小工具
// =============================================================================

/// 主文字色：深可可棕，取色接近应用图标的描边，柔和但足够醒目
const INK: egui::Color32 = egui::Color32::from_rgb(59, 42, 33);
/// 次要文字色（在所有彩色卡片上仍保证 4.5:1 以上的对比度）
const INK_SOFT: egui::Color32 = egui::Color32::from_rgb(108, 86, 72);
/// 卡片底色：奶油白
const CARD: egui::Color32 = egui::Color32::from_rgb(255, 251, 242);
/// 奶油色卡片的描边：暖棕，呼应应用图标的深色轮廓
const CARD_EDGE: egui::Color32 = egui::Color32::from_rgb(226, 188, 150);

/// 左侧统计卡片的浅色底：饱和度提高，和背景、白字都拉开距离
const CARD_WARM: egui::Color32 = egui::Color32::from_rgb(255, 235, 186);
const CARD_SKY: egui::Color32 = egui::Color32::from_rgb(216, 240, 255);
const CARD_MINT: egui::Color32 = egui::Color32::from_rgb(212, 245, 228);
const CARD_LILAC: egui::Color32 = egui::Color32::from_rgb(230, 221, 255);
/// 反馈卡片：游戏中 / 猜对
const CARD_PLAY: egui::Color32 = egui::Color32::from_rgb(255, 238, 190);
const CARD_WIN: egui::Color32 = egui::Color32::from_rgb(206, 244, 220);

/// 与浅色卡片搭配的深色强调文字（对比度均不低于 4.5:1）
const ACCENT_WARM: egui::Color32 = egui::Color32::from_rgb(143, 86, 6);
const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(18, 92, 150);
const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(18, 110, 60);
const ACCENT_RANK: egui::Color32 = egui::Color32::from_rgb(126, 72, 4);

const PINK: egui::Color32 = egui::Color32::from_rgb(255, 138, 168);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 198, 62);
const MINT: egui::Color32 = egui::Color32::from_rgb(72, 199, 137);
const SKY: egui::Color32 = egui::Color32::from_rgb(96, 180, 240);

/// 整窗背景渐变：奶油黄 → 浅粉，对应应用图标的底色
const BG_TOP: egui::Color32 = egui::Color32::from_rgb(255, 243, 213);
const BG_BOTTOM: egui::Color32 = egui::Color32::from_rgb(255, 216, 232);

/// 彩色卡片的描边：把底色压暗一档，得到同色系、更清楚的边框
fn edge_of(fill: egui::Color32) -> egui::Color32 {
    let darken = |v: u8| (v as f32 * 0.85).round() as u8;
    egui::Color32::from_rgb(darken(fill.r()), darken(fill.g()), darken(fill.b()))
}

/// 卡片投影：统一的柔和下落阴影
fn card_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 5],
        blur: 18,
        spread: 0,
        color: egui::Color32::from_rgba_unmultiplied(110, 80, 64, 32),
    }
}

/// 通用卡片 Frame：圆角 + 描边 + 内边距 + 阴影
fn card(fill: egui::Color32) -> egui::Frame {
    let edge = if fill == CARD {
        CARD_EDGE
    } else {
        edge_of(fill)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.5_f32, edge))
        .corner_radius(egui::CornerRadius::same(20))
        .inner_margin(egui::Margin::symmetric(16, 14))
        .shadow(card_shadow())
}

/// 在指定矩形里画垂直渐变色
fn paint_gradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    top: egui::Color32,
    bottom: egui::Color32,
) {
    painter.add(egui::Shape::mesh(gradient_mesh(rect, top, bottom)));
}

/// 生成一块从 top 渐变到 bottom 的四边形网格
fn gradient_mesh(rect: egui::Rect, top: egui::Color32, bottom: egui::Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let base = mesh.vertices.len() as u32;
    for (pos, color) in [
        (rect.left_top(), top),
        (rect.right_top(), top),
        (rect.right_bottom(), bottom),
        (rect.left_bottom(), bottom),
    ] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh
}

/// 四角小闪光，用来点缀背景
fn sparkle(painter: &egui::Painter, center: egui::Pos2, r: f32, color: egui::Color32) {
    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: sparkle_points(center, r),
        closed: true,
        fill: color,
        stroke: egui::Stroke::NONE.into(),
    }));
}

/// 闪光轮廓的 8 个顶点：上下左右四个长角 + 四个内凹点
fn sparkle_points(center: egui::Pos2, r: f32) -> Vec<egui::Pos2> {
    let k = r * 0.24;
    vec![
        center + egui::vec2(0.0, -r),
        center + egui::vec2(k, -k),
        center + egui::vec2(r, 0.0),
        center + egui::vec2(k, k),
        center + egui::vec2(0.0, r),
        center + egui::vec2(-k, k),
        center + egui::vec2(-r, 0.0),
        center + egui::vec2(-k, -k),
    ]
}

/// 整窗背景：奶油色到浅粉色的渐变 + 水彩晕染 + 小星星
fn paint_background(ctx: &egui::Context) {
    let rect = ctx.viewport_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    paint_gradient(&painter, rect, BG_TOP, BG_BOTTOM);

    // 大色块：像绘本里的水彩晕染，饱和度略高，让背景不再灰蒙蒙
    let blobs = [
        (
            egui::pos2(
                rect.left() + rect.width() * 0.05,
                rect.top() + rect.height() * 0.12,
            ),
            rect.width() * 0.22,
            egui::Color32::from_rgba_unmultiplied(255, 198, 110, 34),
        ),
        (
            egui::pos2(
                rect.right() - rect.width() * 0.03,
                rect.top() + rect.height() * 0.30,
            ),
            rect.width() * 0.26,
            egui::Color32::from_rgba_unmultiplied(255, 138, 174, 32),
        ),
        (
            egui::pos2(
                rect.left() + rect.width() * 0.18,
                rect.bottom() + rect.height() * 0.04,
            ),
            rect.width() * 0.24,
            egui::Color32::from_rgba_unmultiplied(104, 188, 244, 30),
        ),
    ];
    for (center, radius, color) in blobs {
        painter.circle_filled(center, radius, color);
    }

    // 固定几颗小星星，位置用比例表示，窗口缩放时一起移动
    let stars = [
        (
            0.055,
            0.10,
            7.0,
            egui::Color32::from_rgba_unmultiplied(255, 196, 92, 150),
        ),
        (
            0.12,
            0.055,
            4.0,
            egui::Color32::from_rgba_unmultiplied(255, 150, 180, 140),
        ),
        (
            0.93,
            0.12,
            6.0,
            egui::Color32::from_rgba_unmultiplied(140, 196, 246, 150),
        ),
        (
            0.885,
            0.06,
            3.5,
            egui::Color32::from_rgba_unmultiplied(255, 196, 92, 140),
        ),
        (
            0.965,
            0.72,
            5.0,
            egui::Color32::from_rgba_unmultiplied(255, 150, 180, 120),
        ),
        (
            0.04,
            0.78,
            4.5,
            egui::Color32::from_rgba_unmultiplied(140, 196, 246, 120),
        ),
    ];
    for (x, y, r, color) in stars {
        sparkle(
            &painter,
            egui::pos2(
                rect.left() + rect.width() * x,
                rect.top() + rect.height() * y,
            ),
            r,
            color,
        );
    }
}

/// 圆形头像：彩色底 + 内置小兔子
fn avatar(ui: &mut egui::Ui, size: f32, bg: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), size * 0.5, bg);
    ui.painter().circle_stroke(
        rect.center(),
        size * 0.5,
        egui::Stroke::new(1.5_f32, egui::Color32::WHITE),
    );
    let inner = egui::Rect::from_center_size(rect.center(), egui::vec2(size * 0.6, size * 0.6));
    egui::Image::new(egui::include_image!("../assets/icons/rabbit.svg")).paint_at(ui, inner);
}

/// 在水平布局里居中一排内容（width 是这一排的预估总宽度）
fn centered_horizontal(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - width) * 0.5).max(0.0));
        add(ui);
    });
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

    /// 随机生成 1 ~ 100 的答案（rand 0.10 的 RangeInclusive 语义）
    fn random_secret() -> u32 {
        rand::rng().random_range(Self::MIN..=Self::MAX)
    }

    /// 创建一局新游戏，所有状态重置，随机生成 secret
    fn new_game() -> Self {
        Self {
            // rand 0.10 的 random_range 是 RangeInclusive（..=）
            secret: Self::random_secret(),
            guess: String::new(),
            attempts: 0,
            last_hint: "在 1 ~ 100 里挑个数字，试试看吧！".into(),
            min_bound: Self::MIN,
            max_bound: Self::MAX,
            won: false,
            shown_excluded: 0.0,
            pop: 0.0,
            confetti: Vec::new(),
            confetti_pending: false,
            audio: Some(Audio::new()),
            store: storage::load(),
            player_dialog: None,
            player_input: String::new(),
            player_error: None,
            player_delete_confirm: false,
        }
    }

    /// 重开一局，保留音频设备和玩家档案
    fn restart(&mut self) {
        let audio = self.audio.take();
        let store = std::mem::take(&mut self.store);
        *self = Self::new_game();
        self.audio = audio;
        self.store = store;
    }

    /// 猜中后把成绩写进当前玩家的档案并落盘
    fn record_win(&mut self) {
        if let Some(player) = self.store.current_player_mut() {
            player.record(self.attempts);
        }
        self.store.save();
    }

    /// 打开「添加玩家 / 修改名字」弹窗
    fn open_player_dialog(&mut self, mode: PlayerDialog) {
        self.player_dialog = Some(mode);
        self.player_error = None;
        self.player_delete_confirm = false;
        self.player_input = match mode {
            PlayerDialog::Add => String::new(),
            PlayerDialog::Rename => self.store.current.clone(),
        };
    }

    /// 「添加玩家 / 修改名字」弹窗
    fn show_player_dialog(&mut self, ctx: &egui::Context) {
        let Some(mode) = self.player_dialog else {
            return;
        };
        let mut close = false;
        let mut confirm = false;
        let mut delete = false;

        egui::Window::new(match mode {
            PlayerDialog::Add => "添加玩家",
            PlayerDialog::Rename => "修改玩家名字",
        })
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_min_width(240.0);
            ui.label(format!("玩家名字（最多 {} 个字）", storage::MAX_NAME_LEN));
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.player_input)
                    .char_limit(storage::MAX_NAME_LEN)
                    .desired_width(f32::INFINITY)
                    .hint_text("例如：妞妞"),
            );
            if resp.changed() {
                self.player_error = None;
            }
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirm = true;
            }
            if let Some(err) = &self.player_error {
                ui.label(
                    egui::RichText::new(err)
                        .size(12.0)
                        .color(egui::Color32::from_rgb(255, 130, 130)),
                );
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("确定").clicked() {
                    confirm = true;
                }
                if ui.button("取消").clicked() {
                    close = true;
                }
                if mode == PlayerDialog::Rename && self.store.players.len() > 1 {
                    let label = if self.player_delete_confirm {
                        "真的删除？"
                    } else {
                        "删除玩家"
                    };
                    if ui.button(label).clicked() {
                        if self.player_delete_confirm {
                            delete = true;
                        } else {
                            self.player_delete_confirm = true;
                        }
                    }
                }
            });
        });

        if close {
            self.player_dialog = None;
            self.player_delete_confirm = false;
            return;
        }

        if delete {
            let index = self.store.current_index();
            self.store.players.remove(index);
            self.store.current = self.store.players[0].name.clone();
            self.store.save();
            self.player_dialog = None;
            self.player_delete_confirm = false;
            self.restart();
            return;
        }

        if confirm {
            let except = (mode == PlayerDialog::Rename).then(|| self.store.current.clone());
            match self
                .store
                .validate_name(&self.player_input, except.as_deref())
            {
                Err(err) => self.player_error = Some(err.message().into()),
                Ok(name) => {
                    match mode {
                        PlayerDialog::Add => {
                            self.store.players.push(storage::Player::new(&name));
                            self.switch_player(name);
                        }
                        PlayerDialog::Rename => {
                            if let Some(player) = self.store.current_player_mut() {
                                player.name = name.clone();
                            }
                            self.store.current = name;
                            self.store.save();
                        }
                    }
                    self.player_dialog = None;
                    self.player_delete_confirm = false;
                }
            }
        }
    }

    /// 切换到指定玩家并开始新的一局
    fn switch_player(&mut self, name: String) {
        self.store.current = name;
        self.store.save();
        self.restart();
    }

    /// 猜对时生成一把彩纸（绘制时再按卡片尺寸换算成像素）
    fn spawn_confetti(&mut self) {
        let palette = [
            egui::Color32::from_rgb(255, 205, 90),
            egui::Color32::from_rgb(110, 235, 165),
            egui::Color32::from_rgb(110, 200, 255),
            egui::Color32::from_rgb(255, 140, 190),
            egui::Color32::from_rgb(190, 165, 255),
        ];
        let mut rng = rand::rng();
        for _ in 0..80 {
            let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.random_range(0.25..0.85);
            self.confetti.push(Confetti {
                // 从卡片顶部中间向上喷出，再被重力拉下
                pos: egui::pos2(0.5, 0.12),
                vel: egui::vec2(angle.cos() * speed, angle.sin() * speed * 0.7 - 0.55),
                angle: rng.random_range(0.0..std::f32::consts::TAU),
                spin: rng.random_range(-8.0..8.0),
                size: rng.random_range(4.0..9.0),
                color: palette[rng.random_range(0..palette.len())],
                life: rng.random_range(1.8..3.0),
            });
        }
    }

    /// 核心猜测逻辑：解析输入、更新状态、给出提示
    /// 用 match 处理 parse 的 Result，再用 Ordering 处理 cmp 的结果
    fn try_guess(&mut self) {
        let trimmed = self.guess.trim();

        // 每次提交都触发一次文字弹跳
        self.pop = 1.0;

        // 先做合法性校验
        match trimmed.parse::<u32>() {
            Ok(n) if n < Self::MIN || n > Self::MAX => {
                self.last_hint = format!("只能选 {}~{} 之间的数字哦", Self::MIN, Self::MAX);
                if let Some(audio) = &self.audio {
                    audio.play_miss();
                }
            }

            Ok(n) => {
                self.attempts += 1;

                // n.cmp(&secret) 返回 Ordering::Less/Greater/Equal
                // match 同时处理缩窄区间与返回提示文本
                let hint = match n.cmp(&self.secret) {
                    std::cmp::Ordering::Less => {
                        // 猜小了 → 下界抬高为 n+1（secret 不可能 ≤ n）
                        self.min_bound = self.min_bound.max(n + 1);
                        "再大一点点！"
                    }
                    std::cmp::Ordering::Greater => {
                        // 猜大了 → 上界压低为 n-1
                        self.max_bound = self.max_bound.min(n - 1);
                        "再小一点点！"
                    }
                    std::cmp::Ordering::Equal => {
                        self.won = true;
                        self.confetti_pending = true;
                        // 记录成绩：最好成绩 / 局数 / 最近记录
                        self.record_win();
                        &format!(
                            "太棒啦！答案就是 {}，你一共猜了 {} 次！",
                            self.secret, self.attempts
                        ) as &str
                    }
                };

                self.last_hint = hint.to_string();

                // 猜对放上行小号角，猜错放两声轻提示音
                if let Some(audio) = &self.audio {
                    if self.won {
                        audio.play_win();
                    } else {
                        audio.play_miss();
                    }
                }
            }

            Err(_) => {
                self.last_hint = "先输入 1 ~ 100 的数字，再点「我猜！」吧～".into();
                if let Some(audio) = &self.audio {
                    audio.play_miss();
                }
            }
        }

        // 无论成功/失败都清空输入框，方便下一次
        self.guess.clear();
    }

    /// 根据 hint 文案返回对应颜色（UI 侧用）
    /// 颜色含义：绿=赢，金黄=非法输入，蓝=要更大，红=要更小
    /// 这些颜色都偏深，保证在浅色卡片上看得清
    fn hint_color(hint: &str) -> egui::Color32 {
        if hint.starts_with("太棒啦") {
            ACCENT_GREEN
        } else if hint.starts_with("只能选") || hint.starts_with("先输入") {
            ACCENT_WARM
        } else if hint.contains("再大") {
            ACCENT_BLUE
        } else if hint.contains("再小") {
            egui::Color32::from_rgb(180, 46, 62)
        } else {
            INK_SOFT
        }
    }
}

/// 清洗输入框内容，保证只能输入 1~100 的整数：
/// - 过滤掉所有非 ASCII 数字字符
/// - 最多保留 3 位
/// - 空串允许（方便退格重输）
/// - 超出 1~100 时返回 None，调用方应保留原来的值
fn sanitize_guess(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut trimmed = digits;
    trimmed.truncate(3);
    if trimmed.is_empty() {
        return Some(trimmed);
    }
    trimmed
        .parse::<u32>()
        .ok()
        .filter(|v| (GuessingGame::MIN..=GuessingGame::MAX).contains(v))
        .map(|_| trimmed)
}

// =============================================================================
// eframe::App 实现 —— 每帧被 egui 调用一次 ui
// =============================================================================

impl eframe::App for GuessingGame {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

impl GuessingGame {
    /// 绘制整帧界面。
    /// 拆成独立方法，测试里不用创建窗口也能直接绘制一帧。
    fn draw(&mut self, ui: &mut egui::Ui) {
        // egui 0.37 起 eframe 的入口是 Ui，需要从 Ui 上取回 Context
        let ctx = ui.ctx().clone();
        // ---------- 动画推进：每帧按真实耗时更新状态 ----------
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);

        // 进度条平滑追赶真实比例，避免一下子跳过去
        let span = (self.max_bound - self.min_bound + 1).max(1) as f32;
        let target = (1.0 - span / (Self::MAX - Self::MIN + 1) as f32) * 100.0;
        self.shown_excluded += (target - self.shown_excluded) * (dt * 9.0).min(1.0);

        // 弹跳强度自然衰减
        self.pop = (self.pop - dt * 3.5).max(0.0);

        // 彩纸：受重力下落 + 旋转 + 逐渐消失
        for c in &mut self.confetti {
            c.vel.y += 1.1 * dt; // 重力
            c.pos += c.vel * dt;
            c.angle += c.spin * dt;
            c.life -= dt;
        }
        self.confetti.retain(|c| c.life > 0.0 && c.pos.y < 1.3);

        // 还有动画在跑就持续重绘，动画结束后自动停止
        if !self.confetti.is_empty()
            || self.pop > 0.01
            || (target - self.shown_excluded).abs() > 0.05
        {
            ctx.request_repaint();
        }

        // 背景画在最底层，面板自身设为透明，让渐变透出来
        paint_background(&ctx);

        // 统一控件风格：圆角、奶油白底、暖棕文字
        ctx.all_styles_mut(|style| {
            let r = egui::CornerRadius::same(15);
            style.visuals.widgets.inactive.corner_radius = r;
            style.visuals.widgets.hovered.corner_radius = r;
            style.visuals.widgets.active.corner_radius = r;
            style.visuals.widgets.open.corner_radius = r;
            style.visuals.widgets.noninteractive.corner_radius = r;

            style.visuals.dark_mode = false;
            style.visuals.override_text_color = Some(INK);
            style.visuals.panel_fill = egui::Color32::TRANSPARENT;
            style.visuals.window_fill = CARD;
            style.visuals.window_stroke = egui::Stroke::new(1.5_f32, CARD_EDGE);
            // 输入框用纯白底 + 更深的描边，和奶油卡片明显区分
            style.visuals.extreme_bg_color = egui::Color32::WHITE;
            style.visuals.selection.bg_fill = PINK.gamma_multiply(0.4);
            style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, INK);

            // 普通按钮：奶油底 + 暖棕描边，悬停时染成浅粉
            style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(255, 240, 208);
            style.visuals.widgets.inactive.bg_stroke =
                egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(226, 190, 152));
            style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(255, 219, 233);
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.8_f32, PINK);
            style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(255, 203, 222);
            style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.8_f32, PINK);

            style.spacing.button_padding = egui::vec2(10.0, 6.0);
        });

        // ---------- 左侧面板：妞妞的战况 ----------
        egui::Panel::left("stats_panel")
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 0)))
            .resizable(true)
            .default_size(238.0)
            .size_range(egui::Rangef::new(210.0, 300.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(16.0);

                        // ---------- 玩家卡片：头像 + 切换 / 设置 ----------
                        card(CARD).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                avatar(ui, 44.0, egui::Color32::from_rgb(255, 224, 235));
                                ui.add_space(8.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("玩家").size(12.5).color(INK_SOFT),
                                    );
                                    ui.label(
                                        egui::RichText::new(&self.store.current)
                                            .size(20.0)
                                            .color(INK)
                                            .strong(),
                                    );
                                });
                            });
                            ui.add_space(10.0);

                            let names: Vec<String> =
                                self.store.players.iter().map(|p| p.name.clone()).collect();
                            let mut selected = self.store.current_index();
                            egui::ComboBox::from_id_salt("player_select")
                                .width(ui.available_width())
                                .selected_text(names[selected].clone())
                                .show_ui(ui, |ui| {
                                    for (i, name) in names.iter().enumerate() {
                                        ui.selectable_value(&mut selected, i, name.clone());
                                    }
                                });
                            // 切换后立刻开新的一局，避免成绩记到别人头上
                            if selected != self.store.current_index() {
                                self.switch_player(names[selected].clone());
                            }

                            ui.add_space(6.0);
                            let button_w = ((ui.available_width() - 8.0) * 0.5).max(80.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add_sized(
                                        [button_w, 30.0],
                                        egui::Button::new(
                                            egui::RichText::new("添加玩家").size(13.5),
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.open_player_dialog(PlayerDialog::Add);
                                }
                                if ui
                                    .add_sized(
                                        [button_w, 30.0],
                                        egui::Button::new(egui::RichText::new("改名").size(13.5)),
                                    )
                                    .clicked()
                                {
                                    self.open_player_dialog(PlayerDialog::Rename);
                                }
                            });
                        });

                        ui.add_space(10.0);

                        // ---------- 次数卡片：数字放大，让小朋友一眼看到 ----------
                        card(CARD_WARM).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.add(icon(
                                    egui::include_image!("../assets/icons/flame.svg"),
                                    18.0,
                                ));
                                ui.label(
                                    egui::RichText::new("已经猜了").size(13.0).color(INK_SOFT),
                                );
                            });
                            // 每次猜完数字会轻微放大再回弹
                            ui.label(
                                egui::RichText::new(format!("{} 次", self.attempts))
                                    .size(30.0 * (1.0 + 0.1 * self.pop))
                                    .color(ACCENT_WARM)
                                    .strong(),
                            );
                        });

                        ui.add_space(10.0);

                        // 提示区间卡片：始终显示，避免猜对后左侧布局突然跳变
                        card(CARD_SKY).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.add(icon(
                                    egui::include_image!("../assets/icons/target.svg"),
                                    18.0,
                                ));
                                ui.label(
                                    egui::RichText::new("还可以猜").size(13.0).color(INK_SOFT),
                                );
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} ~ {}",
                                    self.min_bound, self.max_bound
                                ))
                                .size(22.0)
                                .color(ACCENT_BLUE)
                                .strong(),
                            );
                            ui.add_space(8.0);
                            // 用平滑动画值绘制，进度条会缓缓推进而不是瞬间跳变
                            ui.add(
                                egui::ProgressBar::new(self.shown_excluded / 100.0)
                                    .desired_width(ui.available_width())
                                    .desired_height(12.0)
                                    .corner_radius(egui::CornerRadius::same(6))
                                    .fill(SKY),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "已排除 {:.0}% · 剩 {:.0}%",
                                    self.shown_excluded,
                                    100.0 - self.shown_excluded
                                ))
                                .size(12.5)
                                .color(INK_SOFT),
                            );
                        });

                        ui.add_space(10.0);

                        // ---------- 当前玩家的记录 ----------
                        let (best, wins, recent) = match self.store.current_player() {
                            Some(p) => (
                                p.best,
                                p.wins,
                                p.rounds
                                    .iter()
                                    .take(5)
                                    .map(|v| v.to_string())
                                    .collect::<Vec<_>>()
                                    .join("、"),
                            ),
                            None => (None, 0, String::new()),
                        };
                        card(CARD_MINT).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.add(icon(
                                    egui::include_image!("../assets/icons/star.svg"),
                                    18.0,
                                ));
                                ui.label(
                                    egui::RichText::new("我的记录").size(13.0).color(INK_SOFT),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("最好").size(12.5).color(INK_SOFT));
                                ui.label(
                                    egui::RichText::new(
                                        best.map(|b| format!("{} 次", b))
                                            .unwrap_or_else(|| "—".into()),
                                    )
                                    .size(20.0)
                                    .color(ACCENT_GREEN)
                                    .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("完成 {} 局", wins))
                                        .size(12.5)
                                        .color(INK_SOFT),
                                );
                            });
                            if !recent.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!("最近：{} 次", recent))
                                        .size(12.5)
                                        .color(ACCENT_GREEN),
                                );
                            }
                        });

                        ui.add_space(10.0);

                        // ---------- 排行榜 ----------
                        card(CARD_LILAC).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.add(icon(
                                    egui::include_image!("../assets/icons/trophy.svg"),
                                    18.0,
                                ));
                                ui.label(
                                    egui::RichText::new("排行榜").size(14.0).color(INK).strong(),
                                );
                            });
                            ui.add_space(2.0);

                            let ranking: Vec<(String, Option<u32>)> = self
                                .store
                                .ranking()
                                .iter()
                                .map(|p| (p.name.clone(), p.best))
                                .collect();
                            for (i, (name, best)) in ranking.iter().take(5).enumerate() {
                                let current = *name == self.store.current;
                                let color = if current { ACCENT_RANK } else { INK };
                                let mut text = egui::RichText::new(format!(
                                    "{}. {}　{}",
                                    i + 1,
                                    name,
                                    best.map(|b| format!("{} 次", b))
                                        .unwrap_or_else(|| "—".into())
                                ))
                                .size(13.5)
                                .color(color);
                                if current {
                                    text = text.strong();
                                }
                                ui.label(text);
                            }
                        });

                        // 底部鼓励（猜对后不再需要）
                        if !self.won {
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.add(icon(
                                    egui::include_image!("../assets/icons/flame.svg"),
                                    16.0,
                                ));
                                ui.label(
                                    egui::RichText::new("加油，你一定可以猜到！")
                                        .size(13.0)
                                        .color(INK_SOFT),
                                );
                            });
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("小提示：按回车，或点「我猜！」")
                                    .size(12.5)
                                    .color(INK_SOFT),
                            );
                        }
                        ui.add_space(8.0);
                    });
            });

        // ---------- 玩家设置弹窗 ----------
        self.show_player_dialog(&ctx);

        // ---------- 中央区域：主游戏界面 ----------
        egui::CentralPanel::no_frame()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(18, 0)))
            .show(ui, |ui| {
                ui.add_space(16.0);

                let main_card = egui::Frame::new()
                    .fill(CARD)
                    .stroke(egui::Stroke::new(2.0_f32, CARD_EDGE))
                    .corner_radius(egui::CornerRadius::same(28))
                    .inner_margin(egui::Margin::symmetric(26, 22))
                    .shadow(card_shadow())
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());

                        // ---------- 顶部标题 + 一句引导语 ----------
                        ui.vertical_centered(|ui| {
                            centered_horizontal(ui, 194.0, |ui| {
                                ui.add(icon(
                                    egui::include_image!("../assets/icons/target.svg"),
                                    36.0,
                                ));
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new("妞妞猜数字")
                                        .size(30.0)
                                        .color(INK)
                                        .strong(),
                                );
                            });
                            ui.add_space(4.0);
                            centered_horizontal(ui, 200.0, |ui| {
                                ui.add(icon(
                                    egui::include_image!("../assets/icons/rabbit.svg"),
                                    17.0,
                                ));
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new("妞妞把数字藏好了，快来找找看！")
                                        .size(13.5)
                                        .color(INK_SOFT),
                                );
                            });
                        });
                        ui.add_space(16.0);

                        // ---------- 反馈卡片：猜对时整张卡片变成薄荷绿 ----------
                        let feedback = egui::Frame::new()
                            .corner_radius(egui::CornerRadius::same(24))
                            .inner_margin(egui::Margin::symmetric(20, 20))
                            .fill(if self.won { CARD_WIN } else { CARD_PLAY })
                            .stroke(egui::Stroke::new(
                                2.0_f32,
                                if self.won {
                                    egui::Color32::from_rgb(108, 198, 143)
                                } else {
                                    egui::Color32::from_rgb(238, 186, 72)
                                },
                            ))
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    if self.won {
                                        // ===== 胜利画面 =====
                                        // 猜对的那一帧生成彩纸，之后不再重复生成
                                        if self.confetti_pending {
                                            self.confetti_pending = false;
                                            self.spawn_confetti();
                                        }
                                        ui.add(icon(
                                            egui::include_image!("../assets/icons/trophy.svg"),
                                            52.0,
                                        ));
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "太棒啦！答案是 {}",
                                                self.secret
                                            ))
                                            .size(30.0 * (1.0 + 0.12 * self.pop))
                                            .color(ACCENT_GREEN)
                                            .strong(),
                                        );
                                        ui.add_space(6.0);
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "你一共猜了 {} 次，好厉害！",
                                                self.attempts
                                            ))
                                            .size(17.0)
                                            .color(INK),
                                        );
                                        ui.add_space(16.0);
                                        if ui
                                            .add_sized(
                                                [200.0, 50.0],
                                                egui::Button::image_and_text(
                                                    icon(
                                                        egui::include_image!(
                                                            "../assets/icons/refresh.svg"
                                                        ),
                                                        22.0,
                                                    ),
                                                    egui::RichText::new("再玩一次")
                                                        .size(20.0)
                                                        .color(ACCENT_GREEN)
                                                        .strong(),
                                                )
                                                .corner_radius(egui::CornerRadius::same(25))
                                                .fill(CARD)
                                                .stroke(egui::Stroke::new(2.5_f32, MINT)),
                                            )
                                            .clicked()
                                        {
                                            self.restart();
                                        }
                                    } else {
                                        // ===== 游戏中画面 =====
                                        let color = Self::hint_color(&self.last_hint);
                                        centered_horizontal(ui, 58.0, |ui| {
                                            ui.add(icon(
                                                egui::include_image!("../assets/icons/rabbit.svg"),
                                                15.0,
                                            ));
                                            ui.add_space(5.0);
                                            ui.label(
                                                egui::RichText::new("妞妞说")
                                                    .size(13.0)
                                                    .color(INK_SOFT),
                                            );
                                        });
                                        ui.add_space(6.0);
                                        // 每次提交文字先放大再回弹
                                        ui.label(
                                            egui::RichText::new(&self.last_hint)
                                                .size(24.0 * (1.0 + 0.14 * self.pop))
                                                .color(color)
                                                .strong(),
                                        );
                                        ui.add_space(18.0);

                                        // 输入框 + 「我猜！」按钮（回车也能触发）
                                        ui.horizontal(|ui| {
                                            // 这一排固定宽度 372，居中摆放
                                            ui.add_space(
                                                ((ui.available_width() - 372.0) * 0.5).max(0.0),
                                            );
                                            // 记住编辑前的值，用于拒绝非法输入
                                            let prev_guess = self.guess.clone();
                                            let resp = ui.add_sized(
                                                [212.0, 52.0],
                                                egui::TextEdit::singleline(&mut self.guess)
                                                    .hint_text("1 ~ 100")
                                                    .font(egui::TextStyle::Monospace)
                                                    // 文字与边框留出内边距，光标不再贴边
                                                    .margin(egui::Margin::symmetric(14, 12))
                                                    // 单行文字在 52px 高的框内垂直居中
                                                    .align(egui::Align2::LEFT_CENTER),
                                            );
                                            // 输入框只接受 1~100 的整数：
                                            // 清洗后不合法就退回上一次的合法内容
                                            self.guess =
                                                sanitize_guess(&self.guess).unwrap_or(prev_guess);
                                            // 圆角胶囊按钮：明亮的黄色更吸引小朋友
                                            let guess_btn = ui.add_sized(
                                                [150.0, 52.0],
                                                egui::Button::image_and_text(
                                                    icon(
                                                        egui::include_image!(
                                                            "../assets/icons/dice.svg"
                                                        ),
                                                        24.0,
                                                    ),
                                                    egui::RichText::new("我猜！")
                                                        .size(21.0)
                                                        .color(INK)
                                                        .strong(),
                                                )
                                                .corner_radius(egui::CornerRadius::same(26))
                                                .fill(YELLOW)
                                                .stroke(egui::Stroke::new(
                                                    2.0_f32,
                                                    egui::Color32::from_rgb(214, 152, 24),
                                                )),
                                            );

                                            // 回车或点击「我猜！」都能提交
                                            let submitted = (resp.lost_focus()
                                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                                                || guess_btn.clicked();

                                            if submitted {
                                                self.try_guess();
                                                // 提交后把光标交还输入框，方便连续输入
                                                resp.request_focus();
                                            }
                                        });
                                        ui.add_space(10.0);
                                        ui.label(
                                            egui::RichText::new("按回车也可以提交哦")
                                                .size(12.5)
                                                .color(INK_SOFT),
                                        );
                                    }
                                });
                            });

                        // 彩纸画在反馈卡片最上层
                        if !self.confetti.is_empty() {
                            draw_confetti(ui, feedback.response.rect, &self.confetti);
                        }
                    });

                // 卡片四角的小星星，留给卡片边缘的空白处
                let rect = main_card.response.rect;
                sparkle(
                    ui.painter(),
                    egui::pos2(rect.right() - 14.0, rect.top() + 14.0),
                    7.0,
                    egui::Color32::from_rgba_unmultiplied(255, 190, 50, 190),
                );
                sparkle(
                    ui.painter(),
                    egui::pos2(rect.left() + 14.0, rect.bottom() - 14.0),
                    6.0,
                    egui::Color32::from_rgba_unmultiplied(255, 138, 168, 150),
                );
            });
    }
}

// =============================================================================
// 中文字体加载
// =============================================================================
// egui 默认打包的字体只有拉丁字形，中文会显示为空白/方块 □，
// 所以必须额外提供一个带 CJK 的字体。
//
// 官方文档（epaint 0.37 FontData）：
//   FontData::from_static(bytes) 直接借用静态字节，内部用 Arc 共享
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
        // egui 0.37 的 FontData 内部改用 Arc，from_static 直接借用静态字节
        egui::FontData::from_static(FONT_BYTES).into(),
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
            .with_inner_size([820.0, 640.0]) // 初始窗口大小（放大字号后留足空间）
            .with_min_inner_size([760.0, 540.0]) // 最小可缩大小
            .with_title("妞妞猜数字")
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "guessing-game",
        options,
        Box::new(|cc| {
            // cc = CreationContext，在 App 构造前只调一次
            // 注册图片加载器：PNG / SVG 都走这里，SVG 需要 egui_extras 的 svg 特性
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // 加载中文字体，之后 App 的每帧 update 就都能渲染中文了
            load_chinese_font(&cc.egui_ctx);

            // 界面整体是奶油色 + 彩色卡片的绘本风，固定浅色主题，
            // 否则跟随系统深色主题时输入框、弹窗等控件会变暗
            cc.egui_ctx.set_theme(egui::ThemePreference::Light);

            Ok(Box::<GuessingGame>::default())
        }),
    )
}

// =============================================================================
// 单元测试
// -----------------------------------------------------------------------------
// 测试代码统一放在 src/tests.rs，避免主文件过长。
// =============================================================================

#[cfg(test)]
mod tests;
