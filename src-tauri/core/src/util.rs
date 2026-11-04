use chrono::{Duration, Local, LocalResult, NaiveDate, TimeZone};

pub fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 「学习日」的起点：本地时间 `rollover_hour` 点。
/// 例如 rollover=4 时，凌晨 3 点仍算前一天。
pub fn day_start(now: i64, rollover_hour: u32) -> i64 {
    let date = study_date(now, rollover_hour);
    date_start(date, rollover_hour)
}

/// 某个时间戳所属的学习日（本地日期）
pub fn study_date(ts: i64, rollover_hour: u32) -> NaiveDate {
    let shifted = ts - rollover_hour as i64 * 3600;
    match Local.timestamp_opt(shifted, 0) {
        LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => dt.date_naive(),
        LocalResult::None => chrono::DateTime::from_timestamp(shifted, 0)
            .map(|d| d.date_naive())
            .unwrap_or_default(),
    }
}

/// 某个学习日开始的时间戳
pub fn date_start(date: NaiveDate, rollover_hour: u32) -> i64 {
    let naive = date
        .and_hms_opt(rollover_hour.min(23), 0, 0)
        .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).unwrap());
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => dt.timestamp(),
        LocalResult::None => Local
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest()
            .map(|d| d.timestamp())
            .unwrap_or_else(|| naive.and_utc().timestamp()),
    }
}

/// 去掉 HTML 标签与常见实体，得到纯文本（用于列表展示与搜索）
pub fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let lower = html.to_ascii_lowercase();
    // 去掉 <style>/<script> 的内容
    let cleaned;
    let src = if lower.contains("<style") || lower.contains("<script") {
        cleaned = remove_blocks(html);
        cleaned.as_str()
    } else {
        html
    };
    for c in src.chars() {
        match c {
            '<' => {
                in_tag = true;
                // 块级标签处加空格，避免单词粘连
                out.push(' ');
            }
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            _ => out.push(c),
        }
    }
    let out = decode_entities(&out);
    let out = clean_cloze(&out);
    let out = out.replace("[sound:", " [sound:");
    // 去掉声音标记
    let mut res = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(i) = rest.find("[sound:") {
        res.push_str(&rest[..i]);
        match rest[i..].find(']') {
            Some(j) => rest = &rest[i + j + 1..],
            None => {
                rest = "";
            }
        }
    }
    res.push_str(rest);
    res.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 把填空标记 {{c1::答案::提示}} 还原为「答案」
pub fn clean_cloze(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("{{c") {
        let after = &rest[start + 3..];
        let digits = after.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 || !after[digits..].starts_with("::") {
            out.push_str(&rest[..start + 3]);
            rest = after;
            continue;
        }
        let body_start = start + 3 + digits + 2;
        let Some(end) = rest[body_start..].find("}}") else {
            break;
        };
        let body = &rest[body_start..body_start + end];
        out.push_str(&rest[..start]);
        out.push_str(body.split("::").next().unwrap_or(body));
        rest = &rest[body_start + end + 2..];
    }
    out.push_str(rest);
    out
}

fn remove_blocks(html: &str) -> String {
    let mut s = html.to_string();
    for tag in ["style", "script"] {
        loop {
            let lower = s.to_ascii_lowercase();
            let Some(start) = lower.find(&format!("<{tag}")) else { break };
            let end = lower[start..]
                .find(&format!("</{tag}>"))
                .map(|e| start + e + tag.len() + 3)
                .unwrap_or(s.len());
            s.replace_range(start..end, " ");
        }
    }
    s
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// 把 Anki 媒体文件名清理成安全的本地文件名（防止路径穿越）
pub fn safe_filename(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' | ':' => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        return None;
    }
    Some(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip() {
        assert_eq!(strip_html("<b>apple</b>&nbsp;<i>n.</i>[sound:a.mp3]"), "apple n.");
        assert_eq!(strip_html("<style>.x{}</style>hi<br>there"), "hi there");
        assert_eq!(
            strip_html("The {{c1::cat}} sat on {{c2::mat::地垫}} {{x}}"),
            "The cat sat on mat {{x}}"
        );
    }

    #[test]
    fn safe() {
        assert_eq!(safe_filename("../a.mp3").unwrap(), ".._a.mp3");
        assert!(safe_filename("..").is_none());
    }

    #[test]
    fn day_boundary() {
        let now = now_secs();
        let start = day_start(now, 4);
        assert!(start <= now && now - start < 86400 + 3600);
    }
}
