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

mod audio;
mod storage;

use audio::Audio;
use eframe::egui;
use rand::Rng;
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
            ui.label("玩家名字（最多 8 个字）");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.player_input)
                    .char_limit(8)
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
            let name = self.player_input.trim().to_string();
            if name.is_empty() {
                self.player_error = Some("名字不能为空".into());
                return;
            }
            let except = (mode == PlayerDialog::Rename).then(|| self.store.current.clone());
            if self.store.name_taken(&name, except.as_deref()) {
                self.player_error = Some("这个名字已经有人用了".into());
                return;
            }
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
                self.last_hint = format!("⚠ 只能选 {}~{} 之间的数字哦", Self::MIN, Self::MAX);
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
                            "🎉 太棒啦！答案就是 {}，你一共猜了 {} 次！",
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
                self.last_hint = "⚠ 先输入 1 ~ 100 的数字，再点「我猜！」吧～".into();
                if let Some(audio) = &self.audio {
                    audio.play_miss();
                }
            }
        }

        // 无论成功/失败都清空输入框，方便下一次
        self.guess.clear();
    }

    /// 根据 hint 文案返回对应颜色（UI 侧用）
    /// 颜色含义：绿=赢，黄=非法输入，蓝=要更大，红=要更小
    fn hint_color(hint: &str) -> egui::Color32 {
        if hint.contains("🎉") {
            egui::Color32::from_rgb(110, 235, 165)
        } else if hint.starts_with("⚠") {
            egui::Color32::from_rgb(255, 205, 90)
        } else if hint.contains("再大") {
            egui::Color32::from_rgb(120, 200, 255)
        } else if hint.contains("再小") {
            egui::Color32::from_rgb(255, 130, 130)
        } else {
            egui::Color32::from_rgb(200, 205, 225)
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
// eframe::App 实现 —— 每帧被 egui 调用一次 update
// =============================================================================

impl eframe::App for GuessingGame {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        // 统一把所有控件调圆润一点，更符合儿童应用的观感
        ctx.style_mut(|style| {
            let r = egui::CornerRadius::same(14);
            style.visuals.widgets.inactive.corner_radius = r;
            style.visuals.widgets.hovered.corner_radius = r;
            style.visuals.widgets.active.corner_radius = r;
            style.visuals.widgets.open.corner_radius = r;
            style.visuals.widgets.noninteractive.corner_radius = r;
            // 输入框底色，比页面背景更浅一点，方便小朋友看清
            style.visuals.extreme_bg_color = egui::Color32::from_rgb(24, 28, 42);
        });

        // ---------- 左侧面板：妞妞的战况 ----------
        egui::SidePanel::left("stats_panel")
            .resizable(true)
            .default_width(210.0)
            .width_range(180.0..=280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(12.0);

                        // ---------- 玩家切换 / 设置 ----------
                        ui.label(
                            egui::RichText::new("玩家")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(160, 165, 190)),
                        );
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
                        ui.horizontal(|ui| {
                            if ui.button("＋ 添加玩家").clicked() {
                                self.open_player_dialog(PlayerDialog::Add);
                            }
                            if ui.button("改名").clicked() {
                                self.open_player_dialog(PlayerDialog::Rename);
                            }
                        });

                        ui.add_space(10.0);
                        ui.heading(
                            egui::RichText::new(format!("{} 的战况", self.store.current))
                                .size(20.0)
                                .color(egui::Color32::from_rgb(245, 245, 255)),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "秘密数字藏在 {} ~ {} 里",
                                Self::MIN,
                                Self::MAX
                            ))
                            .size(12.5)
                            .color(egui::Color32::from_rgb(150, 155, 180)),
                        );
                        ui.add_space(14.0);

                        // 次数卡片：数字放大，让小朋友一眼看到
                        egui::Frame::group(ui.style())
                            .corner_radius(egui::CornerRadius::same(14))
                            .inner_margin(egui::Margin::symmetric(14, 12))
                            .fill(egui::Color32::from_rgb(40, 44, 64))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("已经猜了")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(160, 165, 190)),
                                );
                                // 每次猜完数字会轻微放大再回弹
                                ui.label(
                                    egui::RichText::new(format!("{} 次", self.attempts))
                                        .size(28.0 * (1.0 + 0.12 * self.pop))
                                        .color(egui::Color32::from_rgb(255, 205, 90))
                                        .strong(),
                                );
                            });

                        ui.add_space(10.0);

                        // 提示区间卡片：始终显示，避免猜对后左侧布局突然跳变
                        egui::Frame::group(ui.style())
                            .corner_radius(egui::CornerRadius::same(14))
                            .inner_margin(egui::Margin::symmetric(14, 12))
                            .fill(egui::Color32::from_rgb(40, 44, 64))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("还可以猜")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(160, 165, 190)),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} ~ {}",
                                        self.min_bound, self.max_bound
                                    ))
                                    .size(22.0)
                                    .color(egui::Color32::from_rgb(120, 200, 255))
                                    .strong(),
                                );
                                ui.add_space(8.0);
                                // 用平滑动画值绘制，进度条会缓缓推进而不是瞬间跳变
                                ui.add(
                                    egui::ProgressBar::new(self.shown_excluded / 100.0)
                                        .desired_width(ui.available_width())
                                        .desired_height(12.0)
                                        .fill(egui::Color32::from_rgb(110, 235, 165)),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "已排除 {:.0}% · 只剩 {:.0}% 的可能",
                                        self.shown_excluded,
                                        100.0 - self.shown_excluded
                                    ))
                                    .size(12.5)
                                    .color(egui::Color32::from_rgb(150, 240, 190)),
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
                        egui::Frame::group(ui.style())
                            .corner_radius(egui::CornerRadius::same(14))
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .fill(egui::Color32::from_rgb(40, 44, 64))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("我的记录")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(160, 165, 190)),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("最好")
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(150, 155, 180)),
                                    );
                                    ui.label(
                                        egui::RichText::new(
                                            best.map(|b| format!("{} 次", b))
                                                .unwrap_or_else(|| "—".into()),
                                        )
                                        .size(20.0)
                                        .color(egui::Color32::from_rgb(255, 205, 90))
                                        .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("完成 {} 局", wins))
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(180, 185, 210)),
                                    );
                                });
                                if !recent.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!("最近：{} 次", recent))
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(150, 240, 190)),
                                    );
                                }
                            });

                        ui.add_space(10.0);

                        // ---------- 排行榜 ----------
                        ui.label(
                            egui::RichText::new("排行榜")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(245, 245, 255))
                                .strong(),
                        );
                        let ranking: Vec<(String, Option<u32>)> = self
                            .store
                            .ranking()
                            .iter()
                            .map(|p| (p.name.clone(), p.best))
                            .collect();
                        for (i, (name, best)) in ranking.iter().take(5).enumerate() {
                            let current = *name == self.store.current;
                            let color = if current {
                                egui::Color32::from_rgb(255, 205, 90)
                            } else {
                                egui::Color32::from_rgb(190, 195, 220)
                            };
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}. {}　{}",
                                    i + 1,
                                    name,
                                    best.map(|b| format!("{} 次", b))
                                        .unwrap_or_else(|| "—".into())
                                ))
                                .size(13.0)
                                .color(color),
                            );
                        }

                        // 底部操作提示（猜对后不再需要）
                        if !self.won {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new("按回车，或点「🙋 我猜！」")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(130, 135, 160)),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("加油，你一定可以猜到！💪")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(170, 150, 255)),
                            );
                        }
                    });
            });

        // ---------- 玩家设置弹窗 ----------
        self.show_player_dialog(ctx);

        // ---------- 中央区域：主游戏界面 ----------
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::group(ui.style())
                .corner_radius(egui::CornerRadius::same(14))
                .inner_margin(egui::Margin::symmetric(20, 16))
                .show(ui, |ui| {
                    // 顶部标题 + 一句引导语
                    ui.heading(
                        egui::RichText::new("🎯 妞妞猜数字")
                            .size(26.0)
                            .color(egui::Color32::from_rgb(245, 245, 255)),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("🐰 妞妞把数字藏好了，快来找找看！")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(180, 165, 255)),
                    );
                    ui.add_space(12.0);

                    // 深色反馈卡片：猜对时背景变绿色
                    let card = egui::Frame::group(ui.style())
                        .corner_radius(egui::CornerRadius::same(16))
                        .inner_margin(egui::Margin::symmetric(18, 16))
                        .fill(if self.won {
                            egui::Color32::from_rgb(38, 88, 62)
                        } else {
                            egui::Color32::from_rgb(34, 38, 56)
                        })
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                if self.won {
                                    // ===== 胜利画面 =====
                                    // 猜对的那一帧生成彩纸，之后不再重复生成
                                    if self.confetti_pending {
                                        self.confetti_pending = false;
                                        self.spawn_confetti();
                                    }
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "🎉 太棒啦！答案是 {}",
                                            self.secret
                                        ))
                                        .size(30.0 * (1.0 + 0.12 * self.pop))
                                        .color(egui::Color32::from_rgb(120, 240, 150))
                                        .strong(),
                                    );
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "你一共猜了 {} 次，好厉害！",
                                            self.attempts
                                        ))
                                        .size(17.0)
                                        .color(egui::Color32::from_rgb(205, 255, 215)),
                                    );
                                    ui.add_space(12.0);
                                    if ui
                                        .add_sized(
                                            [190.0, 48.0],
                                            egui::Button::new(
                                                egui::RichText::new("🔄 再玩一次")
                                                    .size(20.0)
                                                    .color(egui::Color32::from_rgb(20, 45, 30))
                                                    .strong(),
                                            )
                                            .corner_radius(egui::CornerRadius::same(24))
                                            .fill(egui::Color32::from_rgb(110, 235, 165)),
                                        )
                                        .clicked()
                                    {
                                        self.restart();
                                    }
                                } else {
                                    // ===== 游戏中画面 =====
                                    let color = Self::hint_color(&self.last_hint);
                                    ui.label(
                                        egui::RichText::new("🐰 妞妞说")
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(150, 155, 190)),
                                    );
                                    ui.add_space(2.0);
                                    // 每次提交文字先放大再回弹
                                    ui.label(
                                        egui::RichText::new(&self.last_hint)
                                            .size(22.0 * (1.0 + 0.16 * self.pop))
                                            .color(color)
                                            .strong(),
                                    );
                                    ui.add_space(16.0);

                                    // 输入框 + 「我猜！」按钮（回车也能触发）
                                    ui.horizontal(|ui| {
                                        // 记住编辑前的值，用于拒绝非法输入
                                        let prev_guess = self.guess.clone();
                                        let resp = ui.add_sized(
                                            [210.0, 50.0],
                                            egui::TextEdit::singleline(&mut self.guess)
                                                .hint_text("1 ~ 100")
                                                .font(egui::TextStyle::Monospace)
                                                // 文字与边框留出内边距，光标不再贴边
                                                .margin(egui::Margin::symmetric(12, 10))
                                                // 单行文字在 50px 高的框内垂直居中
                                                .vertical_align(egui::Align::Center),
                                        );
                                        // 输入框只接受 1~100 的整数：
                                        // 清洗后不合法就退回上一次的合法内容
                                        self.guess =
                                            sanitize_guess(&self.guess).unwrap_or(prev_guess);
                                        // 圆角胶囊按钮：明亮的黄色更吸引小朋友
                                        let guess_btn = ui.add_sized(
                                            [140.0, 50.0],
                                            egui::Button::new(
                                                egui::RichText::new("🙋 我猜！")
                                                    .size(20.0)
                                                    .color(egui::Color32::from_rgb(45, 32, 10))
                                                    .strong(),
                                            )
                                            .corner_radius(egui::CornerRadius::same(25))
                                            .fill(egui::Color32::from_rgb(255, 205, 90)),
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
                                }
                            });
                        });

                    // 彩纸画在卡片最上层
                    if !self.confetti.is_empty() {
                        draw_confetti(ui, card.response.rect, &self.confetti);
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
            .with_inner_size([760.0, 620.0]) // 初始窗口大小（放大字号后留足空间）
            .with_min_inner_size([600.0, 500.0]) // 最小可缩大小
            .with_title("妞妞猜数字")
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

// =============================================================================
// 单元测试：输入框清洗逻辑
// =============================================================================

#[cfg(test)]
mod tests {
    use super::sanitize_guess;

    #[test]
    fn accepts_valid_numbers() {
        assert_eq!(sanitize_guess("1"), Some("1".into()));
        assert_eq!(sanitize_guess("42"), Some("42".into()));
        assert_eq!(sanitize_guess("100"), Some("100".into()));
    }

    #[test]
    fn allows_empty_for_backspace() {
        assert_eq!(sanitize_guess(""), Some(String::new()));
    }

    #[test]
    fn rejects_out_of_range() {
        assert_eq!(sanitize_guess("0"), None);
        assert_eq!(sanitize_guess("101"), None);
        assert_eq!(sanitize_guess("999"), None);
        assert_eq!(sanitize_guess("1234"), None); // 截断成 123，仍超范围
    }

    #[test]
    fn strips_non_digit_characters() {
        assert_eq!(sanitize_guess("1a2b"), Some("12".into()));
        assert_eq!(sanitize_guess("abc"), Some(String::new()));
        assert_eq!(sanitize_guess(" 42 "), Some("42".into()));
        assert_eq!(sanitize_guess("１２"), Some(String::new())); // 全角数字不算
        assert_eq!(sanitize_guess("-5"), Some("5".into()));
    }

    #[test]
    fn truncates_to_three_digits() {
        assert_eq!(sanitize_guess("1000"), Some("100".into()));
        assert_eq!(sanitize_guess("1005"), Some("100".into()));
    }

    #[test]
    fn keeps_leading_zeros_when_in_range() {
        // 007 解析为 7，在范围内；保留原样避免输入时光标跳动
        assert_eq!(sanitize_guess("007"), Some("007".into()));
    }
}
