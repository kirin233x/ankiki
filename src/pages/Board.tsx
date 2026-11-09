import { useCallback, useEffect, useMemo, useState } from "react";
import { CalendarDays, ChevronLeft, ChevronRight, Eye, EyeOff, LayoutGrid, Play, Volume2 } from "lucide-react";
import { api, BoardColor, BoardItem, BoardResult, CardDetail } from "../api";
import { useApp } from "../App";
import CardFrame from "../components/CardFrame";
import { Modal, useToast } from "../components/ui";
import { renderSide, stripHtml } from "../render";
import { playSounds, speak } from "../audio";
import { formatDateTime, formatDays, RATING_COLOR, RATING_LABEL } from "../utils";

type Mode = "day" | "week" | "all";

const COLOR_VAR: Record<BoardColor, string> = {
  red: "var(--again)",
  green: "var(--good)",
  blue: "var(--easy)",
  gray: "#a0a4b5",
};

const HIDE_KEY = "ankiki.board.hideMeaning";

function pad(n: number) {
  return String(n).padStart(2, "0");
}

function fmt(d: Date) {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** 以「新的一天开始于」为界的今天 */
function studyToday(rollover: number) {
  return fmt(new Date(Date.now() - rollover * 3600 * 1000));
}

function addDays(date: string, n: number) {
  const d = new Date(date + "T12:00:00");
  d.setDate(d.getDate() + n);
  return fmt(d);
}

function prettyDate(date: string, today: string) {
  if (date === today) return "今天";
  if (date === addDays(today, -1)) return "昨天";
  if (date === addDays(today, -2)) return "前天";
  const [y, m, d] = date.split("-").map(Number);
  const week = "日一二三四五六"[new Date(date + "T12:00:00").getDay()];
  return `${y === Number(today.slice(0, 4)) ? "" : `${y}年`}${m}月${d}日 周${week}`;
}

export default function Board({ initialDate }: { initialDate?: string }) {
  const app = useApp();
  const toast = useToast();
  const today = studyToday(app.settings.rolloverHour);
  const [mode, setMode] = useState<Mode>("day");
  const [date, setDate] = useState(initialDate ?? today);
  const [deckId, setDeckId] = useState<number | null>(null);
  const [filter, setFilter] = useState<"" | BoardColor | "new">("");
  const [sort, setSort] = useState<"time" | "color" | "alpha">("time");
  const [hide, setHide] = useState(() => {
    try {
      return localStorage.getItem(HIDE_KEY) === "1";
    } catch {
      return false;
    }
  });
  const [data, setData] = useState<BoardResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState<BoardItem | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const q =
        mode === "all"
          ? { deckId, from: null, to: null }
          : mode === "week"
            ? { deckId, from: addDays(date, -6), to: date }
            : { deckId, from: date, to: date };
      setData(await api.wordBoard(q));
    } catch (e) {
      toast(String(e), true);
    } finally {
      setLoading(false);
    }
  }, [mode, date, deckId, toast]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => setFilter(""), [mode, date, deckId]);

  const toggleHide = useCallback(() => {
    setHide((h) => {
      try {
        localStorage.setItem(HIDE_KEY, h ? "0" : "1");
      } catch {
        /* 忽略 */
      }
      return !h;
    });
  }, []);

  useEffect(() => {
    const fn = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (open || (t && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA"))) return;
      if (e.key === "h" || e.key === "H") toggleHide();
      else if (mode !== "all" && e.key === "ArrowLeft") setDate((d) => addDays(d, -1));
      else if (mode !== "all" && e.key === "ArrowRight") setDate((d) => (d < today ? addDays(d, 1) : d));
    };
    window.addEventListener("keydown", fn);
    return () => window.removeEventListener("keydown", fn);
  }, [open, mode, today, toggleHide]);

  const items = useMemo(() => {
    if (!data) return [];
    let xs = data.items;
    if (filter === "new") xs = xs.filter((x) => x.isNew);
    else if (filter) xs = xs.filter((x) => x.color === filter);
    const rank: Record<BoardColor, number> = { red: 0, green: 1, blue: 2, gray: 3 };
    if (sort === "color") xs = [...xs].sort((a, b) => rank[a.color] - rank[b.color] || a.firstTs - b.firstTs);
    else if (sort === "alpha") xs = [...xs].sort((a, b) => a.word.localeCompare(b.word));
    return xs;
  }, [data, filter, sort]);

  const isAll = mode === "all";
  const labels: Record<BoardColor, string> = isAll
    ? { red: "需加强", green: "学习中", blue: "已掌握", gray: "熟词" }
    : { red: "重来 / 困难", green: "良好", blue: "简单", gray: "熟词" };

  const title = isAll
    ? "全部已学"
    : mode === "week"
      ? `${prettyDate(addDays(date, -6), today)} – ${prettyDate(date, today)}`
      : prettyDate(date, today);

  return (
    <div className="page">
      <div className="page-inner" style={{ maxWidth: 1240 }}>
        <div className="page-header">
          <div>
            <h1 className="page-title">词表看板</h1>
            <div className="page-subtitle">
              {title}
              {data &&
                (isAll
                  ? ` · 共 ${data.items.length} 个单词`
                  : ` · ${data.items.length} 个单词 · 其中新学 ${data.newCount} 个 · 作答 ${data.reviews} 次`)}
            </div>
          </div>
          <button className={`btn ${hide ? "btn-soft" : ""}`} onClick={toggleHide} title="快捷键 H">
            {hide ? <EyeOff size={16} /> : <Eye size={16} />}
            {hide ? "显示释义" : "隐藏释义自测"}
          </button>
        </div>

        <div className="toolbar">
          <div className="segmented">
            <button className={mode === "day" ? "on" : ""} onClick={() => setMode("day")}>
              某一天
            </button>
            <button className={mode === "week" ? "on" : ""} onClick={() => setMode("week")}>
              近 7 天
            </button>
            <button className={mode === "all" ? "on" : ""} onClick={() => setMode("all")}>
              全部已学
            </button>
          </div>
          {!isAll && (
            <div className="date-nav">
              <button className="icon-btn" onClick={() => setDate(addDays(date, -1))} title="前一天 (←)">
                <ChevronLeft size={17} />
              </button>
              <label className="date-pick">
                <CalendarDays size={15} />
                <input
                  type="date"
                  value={date}
                  max={today}
                  onChange={(e) => e.target.value && setDate(e.target.value)}
                />
              </label>
              <button
                className="icon-btn"
                onClick={() => setDate(addDays(date, 1))}
                disabled={date >= today}
                title="后一天 (→)"
              >
                <ChevronRight size={17} />
              </button>
              {date !== today && (
                <button className="btn btn-ghost" style={{ height: 32 }} onClick={() => setDate(today)}>
                  回到今天
                </button>
              )}
            </div>
          )}
          <div style={{ flex: 1 }} />
          <select
            className="select"
            value={deckId ?? ""}
            onChange={(e) => setDeckId(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">全部词书</option>
            {app.decks.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>
          <select className="select" value={sort} onChange={(e) => setSort(e.target.value as typeof sort)}>
            <option value="time">{isAll ? "按最近复习" : "按学习顺序"}</option>
            <option value="color">按颜色（难词在前）</option>
            <option value="alpha">按字母</option>
          </select>
        </div>

        {data && data.items.length > 0 && (
          <div className="chips">
            <button className={`chip ${filter === "" ? "on" : ""}`} onClick={() => setFilter("")}>
              全部 {data.items.length}
            </button>
            {(["red", "green", "blue", "gray"] as BoardColor[])
              .filter((c) => data[c] > 0 || c !== "gray")
              .map((c) => (
                <button
                  key={c}
                  className={`chip ${filter === c ? "on" : ""}`}
                  onClick={() => setFilter(filter === c ? "" : c)}
                >
                  <i style={{ background: COLOR_VAR[c] }} />
                  {labels[c]} {data[c]}
                </button>
              ))}
            {!isAll && data.newCount > 0 && (
              <button className={`chip ${filter === "new" ? "on" : ""}`} onClick={() => setFilter(filter === "new" ? "" : "new")}>
                <span className="new-tag">新</span>
                新学 {data.newCount}
              </button>
            )}
          </div>
        )}

        {data && data.items.length === 0 && !loading && (
          <div className="empty" style={{ paddingTop: 50 }}>
            <div className="done-badge" style={{ margin: "0 auto 22px", width: 96, height: 96, fontSize: 44 }}>
              <LayoutGrid size={44} color="white" />
            </div>
            <h2>{isAll ? "还没有学过的单词" : `${title}没有背单词`}</h2>
            <p>{isAll ? "开始学习后，这里会列出所有背过的单词。" : "可以换一天看看，或者现在就去背几个吧。"}</p>
            {app.decks.length > 0 && (
              <button className="btn btn-primary btn-lg" onClick={() => app.go({ name: "home" })}>
                <Play size={16} fill="currentColor" />
                去学习
              </button>
            )}
          </div>
        )}

        <div className={`board-grid ${hide ? "hide-meaning" : ""}`}>
          {items.map((it) => (
            <button
              key={it.noteId}
              className={`word-tile ${it.known && it.color === "gray" ? "known" : ""}`}
              style={{ ["--c" as string]: COLOR_VAR[it.color] }}
              onClick={() => setOpen(it)}
              title={isAll ? `间隔 ${formatDays(it.interval)}` : `最差评分：${RATING_LABEL[it.worstRating] ?? ""}`}
            >
              <div className="word-row">
                <span className="word">{it.word || "（空）"}</span>
                {it.isNew && <span className="new-tag">新</span>}
                {!isAll && it.reviews > 1 && <span className="reps">×{it.reviews}</span>}
              </div>
              <div className="meaning">{it.meaning || "—"}</div>
            </button>
          ))}
        </div>
        {items.length > 0 && <div className="board-legend">点击单词查看详情 · H 隐藏/显示释义{!isAll && " · ← → 切换日期"}</div>}
      </div>

      {open && <WordDetail item={open} onClose={() => setOpen(null)} />}
    </div>
  );
}

function WordDetail({ item, onClose }: { item: BoardItem; onClose: () => void }) {
  const app = useApp();
  const [detail, setDetail] = useState<CardDetail | null>(null);
  const [side, setSide] = useState<"q" | "a">("a");

  useEffect(() => {
    api.cardDetail(item.cardId).then(setDetail).catch(() => {});
  }, [item.cardId]);

  const rendered = useMemo(() => (detail ? renderSide(detail.view, side) : null), [detail, side]);

  const play = async () => {
    if (!detail || !rendered) return;
    if (rendered.allSounds.length && (await playSounds(detail.view.notetype.mediaDir, rendered.allSounds))) return;
    speak(stripHtml(item.word));
  };

  return (
    <Modal title={item.word} onClose={onClose} wide>
      <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
        <div className="segmented">
          <button className={side === "q" ? "on" : ""} onClick={() => setSide("q")}>
            正面
          </button>
          <button className={side === "a" ? "on" : ""} onClick={() => setSide("a")}>
            背面
          </button>
        </div>
        <button className="btn" onClick={play}>
          <Volume2 size={15} />
          发音
        </button>
      </div>
      <div className="preview-box" style={{ height: 300 }}>
        {detail && rendered && (
          <CardFrame
            html={rendered.html}
            css={detail.view.notetype.css}
            mediaDir={detail.view.notetype.mediaDir}
            ord={detail.view.ord}
            dark={app.dark}
            styleMode={app.settings.cardStyle}
            compact
          />
        )}
      </div>
      {detail && detail.history.length > 0 && (
        <div className="history" style={{ marginTop: 14, maxHeight: 180, overflowY: "auto" }}>
          {detail.history.slice(0, 20).map((h, i) => (
            <div className="history-row" key={i}>
              <span className="r" style={{ color: RATING_COLOR[h.rating] }}>
                {RATING_LABEL[h.rating]}
              </span>
              <span className="t">{formatDateTime(h.ts)}</span>
              <span style={{ color: "var(--muted)" }}>
                {h.interval > 0 ? formatDays(h.interval) : `${Math.max(1, Math.round(-h.interval / 60))}分钟`}
              </span>
            </div>
          ))}
        </div>
      )}
    </Modal>
  );
}
