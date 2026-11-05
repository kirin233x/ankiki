import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, BookOpen, Check, Eye, EyeOff, Home as HomeIcon, Play, Undo2, Volume2, X } from "lucide-react";
import { api, CardView } from "../api";
import { useApp } from "../App";
import CardFrame from "../components/CardFrame";
import { useToast } from "../components/ui";
import { renderSide, stripHtml } from "../render";
import { playSounds, speak, stopAudio } from "../audio";

interface Done {
  card: CardView;
  known: boolean;
}

const BATCH = 30;

/** 快速过词：认识的标为熟词（不再学习），不认识的加入学习队列最前面 */
export default function Sift({ deckId }: { deckId: number }) {
  const app = useApp();
  const toast = useToast();
  const [queue, setQueue] = useState<CardView[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [remaining, setRemaining] = useState(0);
  const [history, setHistory] = useState<Done[]>([]);
  const [revealed, setRevealed] = useState(false);
  const [flash, setFlash] = useState<"known" | "keep" | null>(null);
  const busy = useRef(false);
  const fetching = useRef(false);
  const deck = app.decks.find((d) => d.id === deckId);
  const { refreshDecks } = app;

  const fetchMore = useCallback(
    async (current: CardView[]) => {
      if (fetching.current) return;
      fetching.current = true;
      try {
        const b = await api.siftBatch(deckId, BATCH, current.map((c) => c.cardId));
        setQueue((q) => {
          const have = new Set(q.map((c) => c.cardId));
          return [...q, ...b.cards.filter((c) => !have.has(c.cardId))];
        });
        setRemaining(b.remaining);
      } catch (e) {
        toast(String(e), true);
      } finally {
        fetching.current = false;
        setLoaded(true);
      }
    },
    [deckId, toast],
  );

  useEffect(() => {
    fetchMore([]);
    return () => {
      stopAudio();
      refreshDecks();
    };
  }, [fetchMore, refreshDecks]);

  const card = queue[0] ?? null;
  const rendered = useMemo(() => (card ? renderSide(card, revealed ? "a" : "q") : null), [card, revealed]);

  useEffect(() => {
    if (!card || !rendered || !app.settings.autoplayAudio) return;
    playSounds(card.notetype.mediaDir, rendered.sounds);
  }, [card, rendered, app.settings.autoplayAudio]);

  const act = useCallback(
    async (known: boolean) => {
      if (!card || busy.current) return;
      busy.current = true;
      setFlash(known ? "known" : "keep");
      try {
        await api.siftAnswer(card.cardId, known);
        const rest = queue.slice(1);
        setQueue(rest);
        setHistory((h) => [...h, { card, known }]);
        setRemaining((r) => Math.max(0, r - 1));
        setRevealed(false);
        if (rest.length < 8) fetchMore(rest);
      } catch (e) {
        toast(String(e), true);
      } finally {
        busy.current = false;
        setTimeout(() => setFlash(null), 160);
      }
    },
    [card, queue, fetchMore, toast],
  );

  const undo = useCallback(async () => {
    const last = history[history.length - 1];
    if (!last || busy.current) return;
    busy.current = true;
    try {
      await api.siftUndo(last.card.cardId, last.card.position);
      setHistory((h) => h.slice(0, -1));
      setQueue((q) => [last.card, ...q.filter((c) => c.cardId !== last.card.cardId)]);
      setRemaining((r) => r + 1);
      setRevealed(false);
    } catch (e) {
      toast(String(e), true);
    } finally {
      busy.current = false;
    }
  }, [history, toast]);

  const replay = useCallback(async () => {
    if (!card || !rendered) return;
    if (rendered.allSounds.length && (await playSounds(card.notetype.mediaDir, rendered.allSounds))) return;
    speak(stripHtml((card.fields[0] ?? "").replace(/\[sound:[^\]]*\]/g, "")));
  }, [card, rendered]);

  const onKey = useCallback(
    (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        if (e.key.toLowerCase() === "z") {
          e.preventDefault();
          undo();
        }
        return;
      }
      switch (e.key) {
        case "1":
        case "ArrowLeft":
          e.preventDefault();
          act(true);
          break;
        case "2":
        case "ArrowRight":
          e.preventDefault();
          act(false);
          break;
        case " ":
        case "Enter":
          e.preventDefault();
          setRevealed((v) => !v);
          break;
        case "z":
        case "Z":
        case "Backspace":
          undo();
          break;
        case "r":
        case "R":
          replay();
          break;
        case "Escape":
          app.go({ name: "home" });
          break;
      }
    },
    [act, undo, replay, app],
  );

  useEffect(() => {
    const fn = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT")) return;
      onKey(e);
    };
    window.addEventListener("keydown", fn);
    return () => window.removeEventListener("keydown", fn);
  }, [onKey]);

  const knownCount = history.filter((h) => h.known).length;
  const keepCount = history.length - knownCount;
  const total = history.length + remaining;
  const pct = total ? (history.length / total) * 100 : 100;

  return (
    <div className="study">
      <div className="study-top" style={{ paddingLeft: 56 }}>
        <button className="btn btn-ghost" onClick={() => app.go({ name: "home" })}>
          <ArrowLeft size={16} />
          返回
        </button>
        <div className="study-title">快速过词 · {deck?.name ?? ""}</div>
        <div className="sift-stat">
          已过 <b>{history.length}</b> / {total}
        </div>
      </div>
      <div className="study-progress">
        <span style={{ width: `${pct}%` }} />
      </div>

      {loaded && !card && (
        <div className="study-done">
          <div className="done-badge">{history.length ? "🎯" : "✨"}</div>
          <h2>{history.length ? "这一轮过完啦！" : "没有需要过的新词"}</h2>
          <p>
            {history.length
              ? "认识的单词已经从学习计划中移除，不认识的会优先出现在每日新词里。"
              : "这本词书的新词都已经过过了。熟词可以在单词本的「熟词」中查看和恢复。"}
          </p>
          {history.length > 0 && (
            <div className="done-stats">
              <div className="done-stat">
                <b style={{ color: "var(--good)" }}>{knownCount}</b>
                <span>标为熟词</span>
              </div>
              <div className="done-stat">
                <b style={{ color: "var(--accent-text)" }}>{keepCount}</b>
                <span>加入学习</span>
              </div>
            </div>
          )}
          <div style={{ display: "flex", gap: 10 }}>
            {history.length > 0 && (
              <button className="btn" onClick={undo}>
                <Undo2 size={16} />
                撤销上一个
              </button>
            )}
            <button className="btn" onClick={() => app.go({ name: "home" })}>
              <HomeIcon size={16} />
              返回首页
            </button>
            <button className="btn btn-primary" onClick={() => app.go({ name: "study", deckId })}>
              <Play size={15} fill="currentColor" />
              开始学习
            </button>
          </div>
        </div>
      )}

      {card && rendered && (
        <div className="study-stage">
          <div className="card-head">
            <div className="sift-session">
              <span style={{ color: "var(--good)" }}>
                <Check size={13} /> 熟词 {knownCount}
              </span>
              <span style={{ color: "var(--accent-text)" }}>
                <BookOpen size={13} /> 待学 {keepCount}
              </span>
            </div>
            <div className="card-tools">
              <button className="icon-btn" title="发音 (R)" onClick={replay}>
                <Volume2 size={16} />
              </button>
              <button
                className="icon-btn"
                title={revealed ? "隐藏释义 (空格)" : "显示释义 (空格)"}
                onClick={() => setRevealed((v) => !v)}
              >
                {revealed ? <EyeOff size={16} /> : <Eye size={16} />}
              </button>
              <button className="icon-btn" title="撤销 (Z)" onClick={undo} disabled={!history.length}>
                <Undo2 size={16} />
              </button>
            </div>
          </div>
          <div
            className={`card-shell sift-shell ${flash ? `flash-${flash}` : ""}`}
            key={`${card.cardId}-${revealed ? "a" : "q"}`}
          >
            <CardFrame
              html={rendered.html}
              css={card.notetype.css}
              mediaDir={card.notetype.mediaDir}
              ord={card.ord}
              dark={app.dark}
              styleMode={app.settings.cardStyle}
              onKey={onKey}
            />
          </div>
          <div className="answer-bar sift-bar">
            <button className="sift-btn known" onClick={() => act(true)}>
              <span className="sift-icon">
                <Check size={20} strokeWidth={2.6} />
              </span>
              <span className="sift-text">
                <b>认识</b>
                <small>标为熟词，不再学习</small>
              </span>
              <kbd>1 / ←</kbd>
            </button>
            <button className="sift-btn keep" onClick={() => act(false)}>
              <span className="sift-icon">
                <X size={20} strokeWidth={2.6} />
              </span>
              <span className="sift-text">
                <b>不认识</b>
                <small>加入学习，优先安排</small>
              </span>
              <kbd>2 / →</kbd>
            </button>
          </div>
          <div className="sift-hint">按空格查看释义 · 熟词随时可以在单词本的「熟词」中恢复</div>
        </div>
      )}
    </div>
  );
}
