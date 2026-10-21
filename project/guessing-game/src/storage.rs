// =============================================================================
// 玩家数据与本地存档
// -----------------------------------------------------------------------------
// 存档位置（自动创建目录）：
//   macOS   ~/Library/Application Support/NiuniuGuess/players.json
//   Windows %APPDATA%\NiuniuGuess\players.json
//   Linux   ~/.local/share/NiuniuGuess/players.json
// 文件是纯 JSON，删掉即恢复初始状态。
// =============================================================================

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 单个玩家的档案
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Player {
    pub name: String,
    /// 历史最好成绩（猜中用的次数，越小越好）
    pub best: Option<u32>,
    /// 猜中过的总局数
    pub wins: u32,
    /// 最近完成的局（猜中次数），新的在最前面，最多保留 20 条
    #[serde(default)]
    pub rounds: Vec<u32>,
}

impl Player {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// 完成一局：更新最好成绩、局数和最近记录
    pub fn record(&mut self, attempts: u32) {
        self.wins += 1;
        self.best = Some(self.best.map_or(attempts, |b| b.min(attempts)));
        self.rounds.insert(0, attempts);
        self.rounds.truncate(20);
    }
}

/// 玩家名字长度上限（按字符数算，一个汉字算一个字）
pub const MAX_NAME_LEN: usize = 8;

/// 玩家名字校验失败的原因
#[derive(Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    Taken,
}

impl NameError {
    /// 给界面显示的提示文案
    pub fn message(&self) -> &'static str {
        match self {
            Self::Empty => "名字不能为空",
            Self::TooLong => "名字最多 8 个字",
            Self::Taken => "这个名字已经有人用了",
        }
    }
}

/// 整个存档
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SaveData {
    /// 当前玩家名字
    pub current: String,
    pub players: Vec<Player>,
}

impl SaveData {
    /// 保证至少有一个玩家，并且 current 指向真实存在的玩家
    pub fn ensure_player(mut self) -> Self {
        if self.players.is_empty() {
            let first = Player::new("妞妞");
            self.current = first.name.clone();
            self.players.push(first);
        } else if !self.players.iter().any(|p| p.name == self.current) {
            self.current = self.players[0].name.clone();
        }
        self
    }

    pub fn current_index(&self) -> usize {
        self.players
            .iter()
            .position(|p| p.name == self.current)
            .unwrap_or(0)
    }

    pub fn current_player(&self) -> Option<&Player> {
        self.players.get(self.current_index())
    }

    pub fn current_player_mut(&mut self) -> Option<&mut Player> {
        let index = self.current_index();
        self.players.get_mut(index)
    }

    /// 排行榜：按最好成绩升序，还没猜中过的玩家排在最后
    pub fn ranking(&self) -> Vec<&Player> {
        let mut list: Vec<&Player> = self.players.iter().collect();
        list.sort_by_key(|p| (p.best.is_none(), p.best.unwrap_or(u32::MAX)));
        list
    }

    /// 名字是否已被其他玩家占用（rename 时排除自己）
    pub fn name_taken(&self, name: &str, except: Option<&str>) -> bool {
        self.players
            .iter()
            .any(|p| p.name == name && Some(p.name.as_str()) != except)
    }

    /// 校验玩家输入的名字，成功返回去掉首尾空白后的名字
    pub fn validate_name(&self, raw: &str, except: Option<&str>) -> Result<String, NameError> {
        let name = raw.trim();
        if name.is_empty() {
            return Err(NameError::Empty);
        }
        if name.chars().count() > MAX_NAME_LEN {
            return Err(NameError::TooLong);
        }
        if self.name_taken(name, except) {
            return Err(NameError::Taken);
        }
        Ok(name.to_string())
    }

    /// 写入磁盘；失败不影响游戏（例如无权限时静默跳过）
    pub fn save(&self) {
        let Some(path) = file_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}

/// 读取存档；文件不存在或损坏时返回带初始玩家的空存档
pub fn load() -> SaveData {
    let data = file_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<SaveData>(&text).ok())
        .unwrap_or_default();
    data.ensure_player()
}

fn file_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("players.json"))
}

#[cfg(target_os = "windows")]
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join("NiuniuGuess"))
}

#[cfg(target_os = "macos")]
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("NiuniuGuess")
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        .map(|base| base.join("NiuniuGuess"))
}

#[cfg(test)]
mod tests {
    use super::{MAX_NAME_LEN, NameError, Player, SaveData};

    /// 造一份有两个玩家的存档
    fn sample() -> SaveData {
        let mut data = SaveData::default();
        data.players.push(Player::new("小A"));
        data.players.push(Player::new("小B"));
        data.current = "小A".into();
        data
    }

    #[test]
    fn validate_name_trims_surrounding_spaces() {
        let data = sample();
        assert_eq!(data.validate_name("  小C  ", None).as_deref(), Ok("小C"));
    }

    #[test]
    fn validate_name_rejects_empty_and_blank() {
        let data = sample();
        assert_eq!(data.validate_name("", None), Err(NameError::Empty));
        assert_eq!(data.validate_name("   ", None), Err(NameError::Empty));
        assert_eq!(data.validate_name("\t\n", None), Err(NameError::Empty));
    }

    #[test]
    fn validate_name_rejects_duplicates() {
        let data = sample();
        assert_eq!(data.validate_name("小A", None), Err(NameError::Taken));
        assert_eq!(data.validate_name(" 小B ", None), Err(NameError::Taken));
    }

    #[test]
    fn validate_name_ignores_self_when_renaming() {
        let data = sample();
        // 改名时把首尾空格去掉后和原名相同，应当允许
        assert_eq!(
            data.validate_name("  小A ", Some("小A")).as_deref(),
            Ok("小A")
        );
    }

    #[test]
    fn validate_name_accepts_exactly_max_length() {
        let data = sample();
        let name = "一".repeat(MAX_NAME_LEN);
        assert_eq!(
            data.validate_name(&name, None).as_deref(),
            Ok(name.as_str())
        );
    }

    #[test]
    fn validate_name_rejects_too_long() {
        let data = sample();
        let name = "一".repeat(MAX_NAME_LEN + 1);
        assert_eq!(data.validate_name(&name, None), Err(NameError::TooLong));
    }

    #[test]
    fn validate_name_counts_characters_not_bytes() {
        // 8 个汉字是 24 字节，按字符数应当通过
        let data = sample();
        let name = "妞".repeat(8);
        assert_eq!(name.len(), 24);
        assert!(data.validate_name(&name, None).is_ok());
    }

    #[test]
    fn name_error_has_message() {
        assert_eq!(NameError::Empty.message(), "名字不能为空");
        assert_eq!(NameError::TooLong.message(), "名字最多 8 个字");
        assert_eq!(NameError::Taken.message(), "这个名字已经有人用了");
    }

    #[test]
    fn first_win_sets_best() {
        let mut p = Player::new("妞妞");
        assert_eq!(p.best, None);
        p.record(8);
        assert_eq!(p.best, Some(8));
        assert_eq!(p.wins, 1);
        assert_eq!(p.rounds, vec![8]);
    }

    #[test]
    fn better_results_lower_best_but_keep_history() {
        let mut p = Player::new("妞妞");
        p.record(9);
        p.record(5);
        p.record(7);
        assert_eq!(p.best, Some(5));
        assert_eq!(p.wins, 3);
        assert_eq!(p.rounds, vec![7, 5, 9]);
    }

    #[test]
    fn history_is_capped_at_twenty() {
        let mut p = Player::new("妞妞");
        for _ in 0..25 {
            p.record(6);
        }
        assert_eq!(p.rounds.len(), 20);
        assert_eq!(p.wins, 25);
    }

    #[test]
    fn ranking_puts_best_first_and_unplayed_last() {
        let mut data = SaveData::default();
        data.players.push(Player::new("小A"));
        data.players.push(Player::new("小B"));
        data.players.push(Player::new("小C"));
        data.players[1].record(4);
        data.players[2].record(6);
        let names: Vec<&str> = data.ranking().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["小B", "小C", "小A"]);
    }

    #[test]
    fn ensure_player_fills_missing_current() {
        let mut data = SaveData::default();
        data.players.push(Player::new("小A"));
        data.current = "不存在".into();
        let data = data.ensure_player();
        assert_eq!(data.current, "小A");
    }

    #[test]
    fn ensure_player_creates_default_profile() {
        let data = SaveData::default().ensure_player();
        assert_eq!(data.players.len(), 1);
        assert_eq!(data.current, "妞妞");
    }

    #[test]
    fn name_taken_ignores_self() {
        let mut data = SaveData::default();
        data.players.push(Player::new("小A"));
        data.players.push(Player::new("小B"));
        assert!(data.name_taken("小B", None));
        assert!(!data.name_taken("小B", Some("小B")));
        assert!(!data.name_taken("小C", None));
    }
}
