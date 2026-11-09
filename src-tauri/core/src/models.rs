use serde::{Deserialize, Serialize};

pub const STATE_NEW: i64 = 0;
pub const STATE_LEARNING: i64 = 1;
pub const STATE_REVIEW: i64 = 2;
pub const STATE_RELEARNING: i64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
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
    /// 词表看板中作为「中文释义」显示的字段名，空表示自动识别
    pub meaning_field: String,
    /// 作为「单词」显示的字段名（单词本、词表看板、排序），空表示自动识别
    pub word_field: String,
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
            meaning_field: String::new(),
            word_field: String::new(),
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
    /// 标为熟词、已从学习中忽略的卡片
    pub known_cards: i64,
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
    /// 新卡在队列中的位置
    pub position: i64,
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
    /// 已存在的单词中，学习进度被更新的卡片数
    pub progress_updated: i64,
}

/// 本机上找到的 Anki 用户
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiProfile {
    pub name: String,
    pub path: String,
    /// 最后修改时间（秒）
    pub modified: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    /// Anki 与 Ankiki 中都有的单词数
    pub matched_notes: i64,
    /// 学习进度被更新的卡片数
    pub updated_cards: i64,
    /// Ankiki 中更新、因此保留未覆盖的卡片数
    pub skipped_newer: i64,
    /// 新增的复习记录条数
    pub revlog_added: i64,
    /// Anki 中有、但 Ankiki 中还没有导入的单词数
    pub missing_notes: i64,
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
    /// 熟词（已忽略）
    pub known: bool,
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
    pub known_cards: i64,
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

/// 快速过词：一批待筛选的新卡
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiftBatch {
    pub cards: Vec<CardView>,
    /// 还没有过的新词数量
    pub remaining: i64,
    /// 本词书新词总数（含已过、不含熟词）
    pub total_new: i64,
    /// 本词书熟词数量
    pub known: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BoardQuery {
    pub deck_id: Option<i64>,
    /// 起始学习日（YYYY-MM-DD，含）；为空表示「全部已学」
    pub from: Option<String>,
    /// 结束学习日（YYYY-MM-DD，含）；为空则与 from 相同
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardItem {
    pub card_id: i64,
    pub note_id: i64,
    pub deck_id: i64,
    pub word: String,
    pub meaning: String,
    /// red | green | blue | gray
    pub color: String,
    /// 在所选时间内是第一次学
    pub is_new: bool,
    pub known: bool,
    /// 所选时间内作答次数
    pub reviews: i64,
    /// 所选时间内最差的一次评分（1~4），「全部已学」时为 0
    pub worst_rating: i64,
    pub interval: i64,
    pub first_ts: i64,
    pub last_ts: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BoardResult {
    pub items: Vec<BoardItem>,
    pub new_count: i64,
    pub red: i64,
    pub green: i64,
    pub blue: i64,
    pub gray: i64,
    pub reviews: i64,
    /// 今天对应的学习日（YYYY-MM-DD）
    pub today: String,
}
