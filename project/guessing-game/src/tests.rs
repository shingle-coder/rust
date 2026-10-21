// =============================================================================
// 单元测试
// -----------------------------------------------------------------------------
// 由 main.rs 末尾的 `#[cfg(test)] mod tests;` 挂载。
// 作为 crate 根的子模块，可以直接访问主文件里的私有常量与纯函数，
// 因此不需要为了测试把任何实现改成 pub。
// =============================================================================

// -----------------------------------------------------------------------------
// 输入框清洗逻辑
// -----------------------------------------------------------------------------
mod input_cleaning {
    use crate::sanitize_guess;

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

// -----------------------------------------------------------------------------
// 界面配色、绘制辅助函数与图标资源
// -----------------------------------------------------------------------------
mod ui {
    use crate::{
        ACCENT_BLUE, ACCENT_GREEN, ACCENT_RANK, ACCENT_WARM, BG_BOTTOM, BG_TOP, CARD, CARD_EDGE,
        CARD_LILAC, CARD_MINT, CARD_PLAY, CARD_SKY, CARD_WARM, CARD_WIN, GuessingGame, INK,
        INK_SOFT, YELLOW, card, edge_of, gradient_mesh, sparkle_points,
    };
    use eframe::egui;
    use eframe::egui::load::{ImageLoader, ImagePoll, SizeHint};
    use egui_extras::loaders::svg_loader::SvgLoader;
    use std::collections::HashSet;

    /// WCAG 2.1 相对亮度
    fn luminance(color: egui::Color32) -> f32 {
        fn channel(value: u8) -> f32 {
            let v = value as f32 / 255.0;
            if v <= 0.039_28 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    /// 两颜色的对比度（1.0 ~ 21.0）
    fn contrast(a: egui::Color32, b: egui::Color32) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// 界面上实际用到的所有卡片底色
    const BACKGROUNDS: [(&str, egui::Color32); 7] = [
        ("奶油白卡片", CARD),
        ("次数卡片", CARD_WARM),
        ("区间卡片", CARD_SKY),
        ("记录卡片", CARD_MINT),
        ("排行榜卡片", CARD_LILAC),
        ("反馈卡片", CARD_PLAY),
        ("胜利卡片", CARD_WIN),
    ];

    #[test]
    fn primary_text_is_readable_on_every_card() {
        for (name, bg) in BACKGROUNDS {
            let ratio = contrast(INK, bg);
            assert!(
                ratio >= 4.5,
                "{name} 上的主文字对比度只有 {ratio:.2}:1，低于 4.5:1"
            );
        }
    }

    #[test]
    fn secondary_text_is_readable_on_every_card() {
        for (name, bg) in BACKGROUNDS {
            let ratio = contrast(INK_SOFT, bg);
            assert!(
                ratio >= 4.5,
                "{name} 上的次要文字对比度只有 {ratio:.2}:1，低于 4.5:1"
            );
        }
    }

    #[test]
    fn page_background_keeps_text_readable() {
        // 底部鼓励语直接画在背景渐变上，不落在卡片里，
        // 所以两端背景色也必须满足 4.5:1
        for (name, bg) in [("渐变顶部", BG_TOP), ("渐变底部", BG_BOTTOM)] {
            for (text_name, fg) in [("主文字", INK), ("次要文字", INK_SOFT)] {
                let ratio = contrast(fg, bg);
                assert!(
                    ratio >= 4.5,
                    "{name}上的{text_name}对比度只有 {ratio:.2}:1，低于 4.5:1"
                );
            }
        }
    }

    #[test]
    fn accent_text_matches_its_card() {
        // 每组都是界面里真实搭配使用的前景 / 背景
        let pairs = [
            ("次数数字", ACCENT_WARM, CARD_WARM),
            ("区间数字", ACCENT_BLUE, CARD_SKY),
            ("记录数字", ACCENT_GREEN, CARD_MINT),
            ("排行榜高亮", ACCENT_RANK, CARD_LILAC),
        ];
        for (name, fg, bg) in pairs {
            let ratio = contrast(fg, bg);
            assert!(ratio >= 4.5, "{name} 的对比度只有 {ratio:.2}:1，低于 4.5:1");
        }
    }

    #[test]
    fn primary_button_text_is_readable() {
        // 「我猜！」按钮：深棕字压在明黄胶囊上
        let ratio = contrast(INK, YELLOW);
        assert!(
            ratio >= 4.5,
            "「我猜！」按钮文字对比度只有 {ratio:.2}:1，低于 4.5:1"
        );
    }

    #[test]
    fn hint_colors_fit_the_feedback_cards() {
        let hints = [
            "太棒啦！答案是 42",
            "只能选 1 ~ 100 之间的整数哦",
            "再大一点点！",
            "再小一点点！",
            "在 1 ~ 100 里挑个数字，试试看吧！",
        ];
        // 提示文字是 24px 加粗，这里按正文标准 4.5:1 检查，
        // 比 WCAG 对大字号要求的 3:1 更严格，和这次"提高对比度"的目标一致
        for hint in hints {
            let color = GuessingGame::hint_color(hint);
            for bg in [CARD_PLAY, CARD_WIN] {
                let ratio = contrast(color, bg);
                assert!(
                    ratio >= 4.5,
                    "提示「{hint}」在主反馈卡片上的对比度只有 {ratio:.2}:1"
                );
            }
        }
    }

    #[test]
    fn hint_colors_are_distinct_per_state() {
        let win = GuessingGame::hint_color("太棒啦！答案是 42");
        let invalid = GuessingGame::hint_color("只能选 1 ~ 100 之间的整数哦");
        let bigger = GuessingGame::hint_color("再大一点点！");
        let smaller = GuessingGame::hint_color("再小一点点！");
        let neutral = GuessingGame::hint_color("在 1 ~ 100 里挑个数字，试试看吧！");
        let colors = [win, invalid, bigger, smaller, neutral];
        let unique: HashSet<[u8; 4]> = colors
            .iter()
            .map(|c| [c.r(), c.g(), c.b(), c.a()])
            .collect();
        assert_eq!(unique.len(), colors.len(), "不同状态应当使用不同颜色");
    }

    #[test]
    fn card_edges_are_darker_than_their_fill() {
        // 卡片描边要比底色深一档，边框才能把卡片从背景里"托"出来
        for (name, fill) in BACKGROUNDS {
            let edge = edge_of(fill);
            assert!(
                luminance(edge) < luminance(fill),
                "{name} 的描边没有比底色更深"
            );
        }
        assert!(luminance(CARD_EDGE) < luminance(CARD));
    }

    #[test]
    fn cards_are_outlined_to_match_the_icon_style() {
        // 应用图标是深色描边 + 高饱和填充，卡片沿用同样的"贴纸"画法：
        // 每张卡片都要有比底色深的描边，否则在浅色背景上会糊成一片
        for (name, fill) in BACKGROUNDS {
            let frame = card(fill);
            assert_eq!(frame.fill, fill, "{name} 的填充色被改动了");
            assert!(
                frame.stroke.width > 0.0,
                "{name} 没有描边，卡片边界会看不清"
            );
            let expected = if fill == CARD {
                CARD_EDGE
            } else {
                edge_of(fill)
            };
            assert_eq!(
                frame.stroke.color, expected,
                "{name} 的描边色应当是同色系压暗后的颜色"
            );
        }
    }

    #[test]
    fn gradient_mesh_spans_the_rect() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 50.0));
        let top = egui::Color32::from_rgb(255, 248, 232);
        let bottom = egui::Color32::from_rgb(255, 228, 238);
        let mesh = gradient_mesh(rect, top, bottom);

        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(mesh.vertices[0].pos, rect.left_top());
        assert_eq!(mesh.vertices[1].pos, rect.right_top());
        assert_eq!(mesh.vertices[2].pos, rect.right_bottom());
        assert_eq!(mesh.vertices[3].pos, rect.left_bottom());
        assert_eq!(mesh.vertices[0].color, top);
        assert_eq!(mesh.vertices[1].color, top);
        assert_eq!(mesh.vertices[2].color, bottom);
        assert_eq!(mesh.vertices[3].color, bottom);
        for vertex in &mesh.vertices {
            assert_eq!(vertex.uv, egui::epaint::WHITE_UV);
        }
    }

    #[test]
    fn sparkle_points_are_symmetric() {
        let center = egui::pos2(50.0, 50.0);
        let r = 8.0;
        let points = sparkle_points(center, r);

        assert_eq!(points.len(), 8);
        for point in &points {
            let distance = (*point - center).length();
            assert!(distance <= r + 0.01, "闪光顶点不应超出半径");
        }
        // 上下左右四个长角
        assert!((points[0] - egui::pos2(50.0, 42.0)).length() < 1e-4);
        assert!((points[2] - egui::pos2(58.0, 50.0)).length() < 1e-4);
        assert!((points[4] - egui::pos2(50.0, 58.0)).length() < 1e-4);
        assert!((points[6] - egui::pos2(42.0, 50.0)).length() < 1e-4);
        // 四个内凹点关于中心对称
        assert!((points[1].x - (100.0 - points[7].x)).abs() < 1e-4);
        assert!((points[1].y - points[7].y).abs() < 1e-4);
        assert!((points[3].x - (100.0 - points[5].x)).abs() < 1e-4);
        assert!((points[3].y - points[5].y).abs() < 1e-4);
    }

    /// 界面里引用的全部 SVG 图标
    const ICONS: [(&str, &[u8]); 8] = [
        ("target", include_bytes!("../assets/icons/target.svg")),
        ("rabbit", include_bytes!("../assets/icons/rabbit.svg")),
        ("dice", include_bytes!("../assets/icons/dice.svg")),
        ("trophy", include_bytes!("../assets/icons/trophy.svg")),
        ("refresh", include_bytes!("../assets/icons/refresh.svg")),
        ("flame", include_bytes!("../assets/icons/flame.svg")),
        ("star", include_bytes!("../assets/icons/star.svg")),
        ("alert", include_bytes!("../assets/icons/alert.svg")),
    ];

    #[test]
    fn every_icon_decodes_to_a_colorful_transparent_image() {
        let ctx = egui::Context::default();
        let loader = SvgLoader::default();

        for (name, bytes) in ICONS {
            let uri = format!("bytes://{name}.svg");
            ctx.include_bytes(uri.clone(), bytes);
            let poll = loader
                .load(&ctx, &uri, SizeHint::default())
                .unwrap_or_else(|err| panic!("{name} 无法被 resvg 解析：{err:?}"));
            let ImagePoll::Ready { image } = poll else {
                panic!("{name} 应当同步加载完成");
            };

            assert_eq!(image.size, [24, 24], "{name} 的 viewBox 应当是 24×24");
            assert!(
                image.pixels.iter().any(|p| p.a() == 0),
                "{name} 应当保留透明背景，不能铺满整块画布"
            );

            // 彩色图标：不透明区域至少出现两种颜色；单色 tint 图标只会有一种
            let opaque_colors: HashSet<[u8; 3]> = image
                .pixels
                .iter()
                .filter(|p| p.a() > 200)
                .map(|p| [p.r(), p.g(), p.b()])
                .collect();
            assert!(
                opaque_colors.len() >= 2,
                "{name} 看起来是单色图标，缺少彩色填充"
            );

            // 中心附近应当有实体图案，而不是只有描边
            let (cx, cy) = (image.size[0] / 2, image.size[1] / 2);
            let has_center = (-2..=2).any(|dy: i32| {
                (-2..=2).any(|dx: i32| {
                    let x = (cx as i32 + dx) as usize;
                    let y = (cy as i32 + dy) as usize;
                    image.pixels[y * image.size[0] + x].a() > 0
                })
            });
            assert!(has_center, "{name} 的中心区域应该有图案");
        }
    }
}
