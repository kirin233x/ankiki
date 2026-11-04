//! FSRS-5 记忆模型（Free Spaced Repetition Scheduler）。
//!
//! 使用官方默认参数，无需训练即可获得比 SM-2 更准确的复习间隔。
//! 参考：https://github.com/open-spaced-repetition/fsrs4anki/wiki/The-Algorithm

pub const W: [f64; 19] = [
    0.40255, 1.18385, 3.173, 15.69105, 7.1949, 0.5345, 1.4604, 0.0046, 1.54575, 0.1192, 1.01925,
    1.9395, 0.11, 0.29605, 2.2698, 0.2315, 2.9898, 0.51655, 0.6621,
];

const DECAY: f64 = -0.5;
const FACTOR: f64 = 19.0 / 81.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Memory {
    /// 稳定性：记忆保持率降到 90% 所需的天数
    pub stability: f64,
    /// 难度：1（最容易）~ 10（最难）
    pub difficulty: f64,
}

/// 经过 `elapsed_days` 天后的回忆概率
pub fn retrievability(elapsed_days: f64, stability: f64) -> f64 {
    (1.0 + FACTOR * elapsed_days.max(0.0) / stability.max(0.01)).powf(DECAY)
}

fn init_stability(rating: u8) -> f64 {
    W[(rating.clamp(1, 4) - 1) as usize].max(0.1)
}

fn init_difficulty(rating: u8) -> f64 {
    (W[4] - (W[5] * (rating as f64 - 1.0)).exp() + 1.0).clamp(1.0, 10.0)
}

fn next_difficulty(d: f64, rating: u8) -> f64 {
    let delta = -W[6] * (rating as f64 - 3.0);
    let d2 = d + delta * (10.0 - d) / 9.0;
    (W[7] * init_difficulty(4) + (1.0 - W[7]) * d2).clamp(1.0, 10.0)
}

fn recall_stability(d: f64, s: f64, r: f64, rating: u8) -> f64 {
    let hard_penalty = if rating == 2 { W[15] } else { 1.0 };
    let easy_bonus = if rating == 4 { W[16] } else { 1.0 };
    s * (1.0
        + W[8].exp()
            * (11.0 - d)
            * s.powf(-W[9])
            * ((W[10] * (1.0 - r)).exp() - 1.0)
            * hard_penalty
            * easy_bonus)
}

fn forget_stability(d: f64, s: f64, r: f64) -> f64 {
    let sf = W[11] * d.powf(-W[12]) * ((s + 1.0).powf(W[13]) - 1.0) * (W[14] * (1.0 - r)).exp();
    sf.min(s)
}

fn short_term_stability(s: f64, rating: u8) -> f64 {
    s * (W[17] * (rating as f64 - 3.0 + W[18])).exp()
}

/// 根据上一次的记忆状态、距今天数和本次评分（1~4）计算新的记忆状态
pub fn next_memory(prev: Option<Memory>, elapsed_days: f64, rating: u8) -> Memory {
    let rating = rating.clamp(1, 4);
    match prev {
        None => Memory {
            stability: init_stability(rating),
            difficulty: init_difficulty(rating),
        },
        Some(m) => {
            let s = if elapsed_days < 1.0 {
                short_term_stability(m.stability, rating)
            } else {
                let r = retrievability(elapsed_days, m.stability);
                if rating == 1 {
                    forget_stability(m.difficulty, m.stability, r)
                } else {
                    recall_stability(m.difficulty, m.stability, r, rating)
                }
            };
            Memory {
                stability: s.clamp(0.01, 36500.0),
                difficulty: next_difficulty(m.difficulty, rating),
            }
        }
    }
}

/// 在期望记忆保持率下的下次间隔（天，未取整）
pub fn interval_days(stability: f64, desired_retention: f64) -> f64 {
    let r = desired_retention.clamp(0.7, 0.99);
    stability / FACTOR * (r.powf(1.0 / DECAY) - 1.0)
}

/// 由 Anki 的 SM-2 难度系数（如 2500 = 250%）粗略换算 FSRS 难度
pub fn difficulty_from_ease(ease_permille: i64) -> f64 {
    if ease_permille <= 0 {
        return 5.0;
    }
    let ease = ease_permille as f64 / 1000.0;
    (10.0 - (ease - 1.3) / (3.0 - 1.3) * 9.0).clamp(1.0, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_equals_stability_at_90() {
        assert!((interval_days(10.0, 0.9) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn good_grows_and_again_shrinks() {
        let m = next_memory(None, 0.0, 3);
        assert!((m.stability - W[2]).abs() < 1e-9);
        let good = next_memory(Some(m), 3.0, 3);
        assert!(good.stability > m.stability);
        let again = next_memory(Some(good), 10.0, 1);
        assert!(again.stability < good.stability);
        assert!(again.difficulty > good.difficulty);
        let easy = next_memory(Some(m), 3.0, 4);
        let hard = next_memory(Some(m), 3.0, 2);
        assert!(easy.stability > good.stability && good.stability > hard.stability);
    }
}
