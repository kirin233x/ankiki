import { useCallback, useEffect, useMemo, useState } from "react";
import { Check, CheckCheck, ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight, Minus, Pause, Play, RotateCcw, Search, Trash2, Undo2, X } from "lucide-react";
import { api, CardDetail, SearchResult } from "../api";
import { useApp } from "../App";
import CardFrame from "../components/CardFrame";
import NoteEditor from "../components/NoteEditor";
import { Confirm, useToast } from "../components/ui";
import { renderSide } from "../render";
import { formatDateTime, formatDays, formatDue, RATING_COLOR, RATING_LABEL, STATE_COLOR, STATE_LABEL } from "../utils";

const PAGE_SIZES = [50, 100, 200, 500];
const PAGE_SIZE_KEY = "ankiki.browse.pageSize";

function loadPageSize(): number {
  try {
    const v = Number(localStorage.getItem(PAGE_SIZE_KEY));
    return PAGE_SIZES.includes(v) ? v : 100;
  } catch {
    return 100;
  }
}

const STATES = [
  { key: "", label: "全部" },
  { key: "due", label: "今日到期", color: "var(--accent)" },
  { key: "new", label: "新卡", color: "var(--new)" },
  { key: "learning", label: "学习中", color: "var(--learn)" },
  { key: "review", label: "复习中", color: "var(--review)" },
  { key: "known", label: "熟词", color: "#a99cff" },
  { key: "suspended", label: "已暂停", color: "var(--faint)" },
];

const COLUMNS: { key: string; label: string; sort?: string; optional?: boolean }[] = [
  { key: "front", label: "单词", sort: "sort" },
  { key: "back", label: "释义" },
  { key: "deck", label: "词书", optional: true },
  { key: "state", label: "状态" },
  { key: "due", label: "到期", sort: "due" },
  { key: "interval", label: "间隔", sort: "interval", optional: true },
  { key: "difficulty", label: "难度", sort: "difficulty", optional: true },
  { key: "reps", label: "复习", sort: "reps", optional: true },
  { key: "lapses", label: "遗忘", sort: "lapses", optional: true },
];

export default function Browse({ initialDeck }: { initialDeck: number | null }) {
  const app = useApp();
  const toast = useToast();
  const [deckId, setDeckId] = useState<number | null>(initialDeck);
  const [text, setText] = useState("");
  const [query, setQuery] = useState("");
  const [state, setState] = useState("");
  const [sort, setSort] = useState("created");
  const [desc, setDesc] = useState(false);
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(loadPageSize);
  const [data, setData] = useState<SearchResult>({ total: 0, rows: [] });
  const [selected, setSelected] = useState<number | null>(null);
  const [detail, setDetail] = useState<CardDetail | null>(null);
  const [side, setSide] = useState<"q" | "a">("q");
  const [confirmDelete, setConfirmDelete] = useState(false);
  // 批量选择
  const [checked, setChecked] = useState<Set<number>>(new Set());
  const [allMatching, setAllMatching] = useState(false);
  const [lastIndex, setLastIndex] = useState<number | null>(null);
  const [confirmBatchDelete, setConfirmBatchDelete] = useState(false);

  // 筛选条件变化时清空选择
  useEffect(() => {
    setChecked(new Set());
    setAllMatching(false);
    setLastIndex(null);
  }, [deckId, query, state]);

  // 输入防抖
  useEffect(() => {
    const t = setTimeout(() => {
      setQuery(text);
      setPage(0);
    }, 250);
    return () => clearTimeout(t);
  }, [text]);

  const load = useCallback(async () => {
    try {
      setData(
        await api.searchCards({ deckId, text: query, state, sort, desc, offset: page * pageSize, limit: pageSize }),
      );
    } catch (e) {
      toast(String(e), true);
    }
  }, [deckId, query, state, sort, desc, page, pageSize, toast]);

  useEffect(() => {
    load();
  }, [load]);

  const loadDetail = useCallback(async (id: number) => {
    try {
      setDetail(await api.cardDetail(id));
    } catch {
      setDetail(null);
      setSelected(null);
    }
  }, []);

  useEffect(() => {
    if (selected != null) loadDetail(selected);
    else setDetail(null);
    setSide("q");
  }, [selected, loadDetail]);

  const refreshAll = async () => {
    await load();
    if (selected != null) await loadDetail(selected);
    app.refreshDecks();
  };

  const rendered = useMemo(() => (detail ? renderSide(detail.view, side) : null), [detail, side]);
  const pages = Math.max(1, Math.ceil(data.total / pageSize));

  const pageIds = data.rows.map((r) => r.cardId);
  const pageAllChecked = pageIds.length > 0 && pageIds.every((id) => checked.has(id));
  const pageSomeChecked = pageIds.some((id) => checked.has(id));

  const toggleRow = (index: number, shift: boolean) => {
    const id = data.rows[index].cardId;
    const next = new Set(checked);
    const on = !next.has(id);
    if (shift && lastIndex != null) {
      const [a, b] = lastIndex < index ? [lastIndex, index] : [index, lastIndex];
      for (let i = a; i <= b; i++) on ? next.add(data.rows[i].cardId) : next.delete(data.rows[i].cardId);
    } else {
      on ? next.add(id) : next.delete(id);
    }
    setChecked(next);
    setAllMatching(false);
    setLastIndex(index);
  };

  const togglePage = () => {
    const next = new Set(checked);
    if (pageAllChecked) pageIds.forEach((id) => next.delete(id));
    else pageIds.forEach((id) => next.add(id));
    setChecked(next);
    setAllMatching(false);
  };

  const selectAllMatching = async () => {
    try {
      const ids = await api.searchCardIds({ deckId, text: query, state });
      setChecked(new Set(ids));
      setAllMatching(true);
    } catch (e) {
      toast(String(e), true);
    }
  };

  const clearChecked = () => {
    setChecked(new Set());
    setAllMatching(false);
  };

  const batch = async (fn: (ids: number[]) => Promise<void>, msg: string) => {
    const ids = [...checked];
    try {
      await fn(ids);
      toast(msg.replace("{n}", String(ids.length)));
      clearChecked();
      refreshAll();
    } catch (e) {
      toast(String(e), true);
    }
  };

  const toggleSort = (key?: string) => {
    if (!key) return;
    if (sort === key) setDesc(!desc);
    else {
      setSort(key);
      setDesc(false);
    }
    setPage(0);
  };

  return (
    <div className={`browse ${detail ? "has-drawer" : ""}`}>
      <div className="browse-main">
        <div className="page-header" style={{ margin: "8px 0 18px" }}>
          <div>
            <h1 className="page-title">单词本</h1>
            <div className="page-subtitle">共 {data.total} 张卡片</div>
          </div>
        </div>
        <div className="toolbar">
          <div className="search-box">
            <Search size={16} />
            <input
              className="input"
              placeholder="搜索单词、释义或标签…"
              value={text}
              onChange={(e) => setText(e.target.value)}
            />
          </div>
          <select
            className="select"
            value={deckId ?? ""}
            onChange={(e) => {
              setDeckId(e.target.value ? Number(e.target.value) : null);
              setPage(0);
            }}
          >
            <option value="">全部词书</option>
            {app.decks.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>
        </div>
        <div className="chips">
          {STATES.map((s) => (
            <button
              key={s.key}
              className={`chip ${state === s.key ? "on" : ""}`}
              onClick={() => {
                setState(s.key);
                setPage(0);
              }}
            >
              {s.color && <i style={{ background: s.color }} />}
              {s.label}
            </button>
          ))}
        </div>
        {checked.size > 0 && (
          <div className="batch-bar">
            <span className="count">已选 {checked.size} 张</span>
            <button className="btn" onClick={() => batch((ids) => api.setKnown(ids, true), "已将 {n} 张标为熟词")}>
              <CheckCheck size={14} />
              标为熟词
            </button>
            <button className="btn" onClick={() => batch((ids) => api.setKnown(ids, false), "已取消 {n} 张熟词")}>
              <Undo2 size={14} />
              取消熟词
            </button>
            <button className="btn" onClick={() => batch((ids) => api.setSuspended(ids, true), "已暂停 {n} 张")}>
              <Pause size={14} />
              暂停
            </button>
            <button className="btn" onClick={() => batch((ids) => api.setSuspended(ids, false), "已恢复 {n} 张")}>
              <Play size={14} />
              恢复
            </button>
            <button className="btn" onClick={() => batch((ids) => api.forgetCards(ids), "已重置 {n} 张")}>
              <RotateCcw size={14} />
              重置进度
            </button>
            <button className="btn btn-danger" onClick={() => setConfirmBatchDelete(true)}>
              <Trash2 size={14} />
              删除
            </button>
            <button className="icon-btn" title="取消选择" onClick={clearChecked}>
              <X size={16} />
            </button>
          </div>
        )}
        {pageAllChecked && !allMatching && data.total > data.rows.length && (
          <div className="select-all-tip">
            已选中本页 {data.rows.length} 张。
            <button onClick={selectAllMatching}>选择全部 {data.total} 张符合条件的卡片</button>
          </div>
        )}
        <div className="table-wrap">
          <table className="cards">
            <thead>
              <tr>
                <th className="c-check" onClick={togglePage}>
                  <span className={`cbox ${pageAllChecked ? "on" : pageSomeChecked ? "some" : ""}`}>
                    {pageAllChecked ? <Check size={12} strokeWidth={3} /> : pageSomeChecked ? <Minus size={12} strokeWidth={3} /> : null}
                  </span>
                </th>
                {COLUMNS.map((c) => (
                  <th
                    key={c.key}
                    className={`c-${c.key} ${c.optional ? "col-optional" : ""}`}
                    onClick={() => toggleSort(c.sort)}
                    style={{ cursor: c.sort ? "pointer" : "default" }}
                  >
                    {c.label}
                    {c.sort && sort === c.sort && <span className="sort-ind">{desc ? "▼" : "▲"}</span>}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data.rows.map((r, i) => (
                <tr
                  key={r.cardId}
                  className={selected === r.cardId || checked.has(r.cardId) ? "selected" : ""}
                  onClick={(e) => {
                    if (e.shiftKey || e.metaKey || e.ctrlKey) toggleRow(i, e.shiftKey);
                    else setSelected(r.cardId === selected ? null : r.cardId);
                  }}
                  style={{ opacity: r.suspended || r.known ? 0.55 : 1 }}
                >
                  <td
                    className="cell-check"
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleRow(i, e.shiftKey);
                    }}
                  >
                    <span className={`cbox ${checked.has(r.cardId) ? "on" : ""}`}>
                      {checked.has(r.cardId) && <Check size={12} strokeWidth={3} />}
                    </span>
                  </td>
                  <td className="front" title={r.front}>
                    {r.front || "（空）"}
                  </td>
                  <td className="back" title={r.back}>
                    {r.back}
                  </td>
                  <td className="num col-optional" style={{ maxWidth: 140 }} title={r.deckName}>
                    {r.deckName}
                  </td>
                  <td>
                    <span
                      className="state-dot"
                      style={{ color: r.known ? "#8b7bff" : r.suspended ? "var(--faint)" : STATE_COLOR[r.state] }}
                    >
                      <i style={{ background: "currentColor" }} />
                      {r.known ? "熟词" : r.suspended ? "已暂停" : STATE_LABEL[r.state]}
                    </span>
                  </td>
                  <td className="num">{r.known ? "—" : formatDue(r.due, r.state)}</td>
                  <td className="num col-optional">{r.state === 0 ? "—" : formatDays(r.interval)}</td>
                  <td className="num col-optional">
                    {r.difficulty > 0 ? <DifficultyBar value={r.difficulty} /> : "—"}
                  </td>
                  <td className="num col-optional">{r.reps}</td>
                  <td className="num col-optional">{r.lapses}</td>
                </tr>
              ))}
              {data.rows.length === 0 && (
                <tr>
                  <td colSpan={COLUMNS.length + 1} style={{ textAlign: "center", padding: 60, color: "var(--muted)" }}>
                    没有找到符合条件的卡片
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        <div className="pager">
          <span className="pager-total">共 {data.total} 张</span>
          <div className="pager-btns">
            <span className="pager-label">每页</span>
            <select
              className="select pager-select"
              value={pageSize}
              onChange={(e) => {
                const v = Number(e.target.value);
                setPageSize(v);
                setPage(0);
                try {
                  localStorage.setItem(PAGE_SIZE_KEY, String(v));
                } catch {
                  /* 忽略 */
                }
              }}
            >
              {PAGE_SIZES.map((n) => (
                <option key={n} value={n}>
                  {n} 个
                </option>
              ))}
            </select>
            <button className="btn pager-edge" disabled={page === 0} onClick={() => setPage(0)} title="第一页">
              <ChevronsLeft size={16} />
            </button>
            <button className="btn" disabled={page === 0} onClick={() => setPage(page - 1)} title="上一页">
              <ChevronLeft size={16} />
            </button>
            <span className="pager-label">
              {page + 1} / {pages}
            </span>
            <button className="btn" disabled={page + 1 >= pages} onClick={() => setPage(page + 1)} title="下一页">
              <ChevronRight size={16} />
            </button>
            <button
              className="btn pager-edge"
              disabled={page + 1 >= pages}
              onClick={() => setPage(pages - 1)}
              title="最后一页"
            >
              <ChevronsRight size={16} />
            </button>
          </div>
        </div>
      </div>

      {detail && rendered && (
        <aside className="drawer">
          <div className="drawer-head">
            <h3>{detail.row.front}</h3>
            <div className="segmented">
              <button className={side === "q" ? "on" : ""} onClick={() => setSide("q")}>
                正面
              </button>
              <button className={side === "a" ? "on" : ""} onClick={() => setSide("a")}>
                背面
              </button>
            </div>
            <button className="icon-btn" onClick={() => setSelected(null)}>
              <X size={18} />
            </button>
          </div>
          <div className="drawer-body">
            <div className="preview-box">
              <CardFrame
                html={rendered.html}
                css={detail.view.notetype.css}
                mediaDir={detail.view.notetype.mediaDir}
                ord={detail.view.ord}
                dark={app.dark}
                compact
              />
            </div>
            <div className="info-grid">
              <Info
                label="状态"
                value={detail.row.known ? "熟词" : detail.row.suspended ? "已暂停" : STATE_LABEL[detail.row.state]}
              />
              <Info label="下次复习" value={formatDue(detail.row.due, detail.row.state)} />
              <Info label="当前间隔" value={detail.row.state === 0 ? "—" : formatDays(detail.row.interval)} />
              <Info
                label="当前记忆率"
                value={detail.retrievability != null ? `${Math.round(detail.retrievability * 100)}%` : "—"}
              />
              <Info label="稳定性" value={detail.row.stability > 0 ? formatDays(Math.round(detail.row.stability)) : "—"} />
              <Info label="难度" value={detail.row.difficulty > 0 ? `${detail.row.difficulty.toFixed(1)} / 10` : "—"} />
              <Info label="复习次数" value={String(detail.row.reps)} />
              <Info label="遗忘次数" value={String(detail.row.lapses)} />
            </div>

            <div className="drawer-section">编辑内容</div>
            <NoteEditor card={detail.view} onSaved={refreshAll} />

            {detail.history.length > 0 && (
              <>
                <div className="drawer-section">复习记录</div>
                <div className="history">
                  {detail.history.map((h, i) => (
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
              </>
            )}
            <div style={{ fontSize: 11.5, color: "var(--faint)", marginTop: 16 }}>
              添加于 {formatDateTime(detail.createdAt * 1000)} · 笔记类型「{detail.view.notetype.name}」
            </div>
          </div>
          <div className="drawer-actions">
            <button
              className="btn"
              onClick={async () => {
                await api.setKnown([detail.row.cardId], !detail.row.known);
                toast(detail.row.known ? "已取消熟词" : "已标为熟词");
                refreshAll();
              }}
            >
              {detail.row.known ? <Undo2 size={15} /> : <CheckCheck size={15} />}
              {detail.row.known ? "取消熟词" : "熟词"}
            </button>
            <button
              className="btn"
              onClick={async () => {
                await api.setSuspended([detail.row.cardId], !detail.row.suspended);
                toast(detail.row.suspended ? "已恢复" : "已暂停");
                refreshAll();
              }}
            >
              {detail.row.suspended ? <Play size={15} /> : <Pause size={15} />}
              {detail.row.suspended ? "恢复" : "暂停"}
            </button>
            <button
              className="btn"
              disabled={detail.row.state === 0}
              title="重置为新卡，清空学习进度"
              onClick={async () => {
                await api.forgetCards([detail.row.cardId]);
                toast("已重置为新卡");
                refreshAll();
              }}
            >
              <RotateCcw size={15} />
              重置
            </button>
            <button className="btn btn-danger" title="删除" onClick={() => setConfirmDelete(true)}>
              <Trash2 size={15} />
            </button>
          </div>
        </aside>
      )}

      {confirmBatchDelete && (
        <Confirm
          title={`删除 ${checked.size} 张卡片？`}
          danger
          confirmText="删除"
          message="选中卡片所属的单词及其全部卡片和复习记录将被永久删除，无法恢复。"
          onClose={() => setConfirmBatchDelete(false)}
          onConfirm={() => {
            setSelected(null);
            batch((ids) => api.deleteCards(ids), "已删除 {n} 张");
          }}
        />
      )}
      {confirmDelete && detail && (
        <Confirm
          title="删除这个单词？"
          danger
          confirmText="删除"
          message={`「${detail.row.front}」及其所有卡片和复习记录将被永久删除。`}
          onClose={() => setConfirmDelete(false)}
          onConfirm={async () => {
            await api.deleteNotes([detail.row.noteId]);
            setSelected(null);
            toast("已删除");
            refreshAll();
          }}
        />
      )}
    </div>
  );
}

function Info({ label, value }: { label: string; value: string }) {
  return (
    <div className="info-item">
      <span>{label}</span>
      <b>{value}</b>
    </div>
  );
}

function DifficultyBar({ value }: { value: number }) {
  const pct = ((value - 1) / 9) * 100;
  const color = value < 4 ? "var(--good)" : value < 7 ? "var(--hard)" : "var(--again)";
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
      <span style={{ width: 40, height: 5, borderRadius: 3, background: "var(--panel-3)", overflow: "hidden" }}>
        <span style={{ display: "block", width: `${pct}%`, height: "100%", background: color }} />
      </span>
      {value.toFixed(1)}
    </span>
  );
}
