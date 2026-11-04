use serde::{Deserialize, Serialize};

pub const STATE_NEW: i64 = 0;
pub const STATE_LEARNING: i64 = 1;
pub const STATE_REVIEW: i64 = 2;
pub const STATE_RELEARNING: i64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckConfig {
    pub new_per_day: i64,
    pub reviews_per_day: i64,
    pub desired_retention: f64,
    /// 学习步长（分钟），如 [1, 10]
    pub learning_steps: Vec<f64>,
    /// 重学步长（分钟），如 [10]
    pub relearning_steps: Vec<f64>,
    pub max_interval: i64,
    /// 新卡顺序：0 按词书顺序，1 随机
    pub new_order: i64,
}

impl Default for DeckConfig {
    fn default() -> Self {
        Self {
            new_per_day: 20,
            reviews_per_day: 200,
            desired_retention: 0.9,
            learning_steps: vec![1.0, 10.0],
            relearning_steps: vec![10.0],
            max_interval: 36500,
            new_order: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub new: i64,
    pub learning: i64,
    pub review: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: i64,
    pub config: DeckConfig,
    pub due: Counts,
    pub total_cards: i64,
    pub new_cards: i64,
    pub learned_cards: i64,
    pub mature_cards: i64,
    pub suspended_cards: i64,
    pub studied_today: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDef {
    pub name: String,
    pub qfmt: String,
    pub afmt: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotetypeView {
    pub id: i64,
    pub name: String,
    /// 0 普通，1 填空（cloze）
    pub kind: i64,
    pub css: String,
    pub fields: Vec<String>,
    pub templates: Vec<TemplateDef>,
    /// 媒体文件所在目录（绝对路径）
    pub media_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardView {
    pub card_id: i64,
    pub note_id: i64,
    pub deck_id: i64,
    pub deck_name: String,
    pub ord: i64,
    pub state: i64,
    pub fields: Vec<String>,
    pub tags: String,
    pub notetype: NotetypeView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyState {
    pub card: Option<CardView>,
    pub counts: Counts,
    /// 四个按钮对应的下次间隔文字，如 ["1分钟", "6分钟", "10分钟", "4天"]
    pub previews: Vec<String>,
    /// 今天还有学习中的卡片但尚未到期时，最近一张的到期时间
    pub next_due_at: Option<i64>,
    pub studied_today: i64,
    pub can_undo: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub stage: String,
    pub current: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub deck_id: i64,
    pub deck_name: String,
    pub notes_added: i64,
    pub cards_added: i64,
    pub duplicates: i64,
    pub media_files: i64,
    pub with_progress: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// auto | light | dark
    pub theme: String,
    pub rollover_hour: u32,
    pub autoplay_audio: bool,
    /// original：保留词书自带样式；ankiki：使用 Ankiki 美化样式
    pub card_style: String,
    pub show_intervals: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "auto".into(),
            rollover_hour: 4,
            autoplay_audio: true,
            card_style: "original".into(),
            show_intervals: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub deck_id: Option<i64>,
    #[serde(default)]
    pub text: String,
    /// all | new | learning | review | suspended | due
    #[serde(default)]
    pub state: String,
    /// sort | due | interval | difficulty | reps | lapses | created
    #[serde(default)]
    pub sort: String,
    #[serde(default)]
    pub desc: bool,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardRow {
    pub card_id: i64,
    pub note_id: i64,
    pub deck_id: i64,
    pub deck_name: String,
    pub front: String,
    pub back: String,
    pub state: i64,
    pub due: i64,
    pub interval: i64,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    pub suspended: bool,
    pub tags: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub total: i64,
    pub rows: Vec<CardRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevlogEntry {
    pub ts: i64,
    pub rating: i64,
    pub state: i64,
    pub interval: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDetail {
    pub view: CardView,
    pub row: CardRow,
    pub retrievability: Option<f64>,
    pub last_review: i64,
    pub created_at: i64,
    pub history: Vec<RevlogEntry>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DayCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub today_reviews: i64,
    pub today_new: i64,
    pub today_time_ms: i64,
    pub today_again: i64,
    pub streak: i64,
    pub total_reviews: i64,
    pub total_cards: i64,
    pub new_cards: i64,
    pub learning_cards: i64,
    pub young_cards: i64,
    pub mature_cards: i64,
    pub suspended_cards: i64,
    pub retention_30d: Option<f64>,
    pub avg_daily: f64,
    pub days_studied: i64,
    pub heatmap: Vec<DayCount>,
    pub forecast: Vec<i64>,
    /// 按评分统计的次数（重来/困难/良好/简单），近 30 天
    pub ratings_30d: Vec<i64>,
    /// 间隔分布：<1d, 1-3d, 3-7d, 1-2w, 2-4w, 1-3m, 3-6m, 6m+
    pub intervals: Vec<i64>,
}
