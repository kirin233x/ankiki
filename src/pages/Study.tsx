import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, BarChart3, CheckCheck, Clock, Home as HomeIcon, Pause, Pencil, Undo2, Volume2 } from "lucide-react";
import { api, StudyState } from "../api";
import { useApp } from "../App";
import CardFrame from "../components/CardFrame";
import NoteEditor from "../components/NoteEditor";
import { Modal, useToast } from "../components/ui";
import { renderSide, stripHtml } from "../render";
import { playSounds, speak, stopAudio } from "../audio";
import { STATE_COLOR, STATE_LABEL } from "../utils";

const RATINGS = [
  { r: 1, cls: "again", label: "重来", key: "1" },
  { r: 2, cls: "hard", label: "困难", key: "2" },
  { r: 3, cls: "good", label: "良好", key: "3" },
  { r: 4, cls: "easy", label: "简单", key: "4" },
];

export default function Study({ deckId }: { deckId: number }) {
  const app = useApp();
  const toast = useToast();
  const [st, setSt] = useState<StudyState | null>(null);
  const [revealed, setRevealed] = useState(false);
  const [typed, setTyped] = useState("");
  const [pressed, setPressed] = useState<number | null>(null);
  const [editing, setEditing] = useState(false);
  const [sessionCount, setSessionCount] = useState(0);
  const [now, setNow] = useState(Date.now());
  const shownAt = useRef(Date.now());
  const busy = useRef(false);
  const deck = app.decks.find((d) => d.id === deckId);
  const { refreshDecks } = app;

  const load = useCallback(async () => {
    try {
      setSt(await api.studyState(deckId));
      setRevealed(false);
      setTyped("");
    } catch (e) {
      toast(String(e), true);
    }
  }, [deckId, toast]);

  useEffect(() => {
    load();
    return () => {
      stopAudio();
      refreshDecks();
    };
  }, [load, refreshDecks]);

  const card = st?.card ?? null;
  const rendered = useMemo(
    () => (card ? renderSide(card, revealed ? "a" : "q", typed) : null),
    // typed 只在翻面时使用
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [card, revealed],
  );

  // 自动播放发音
  useEffect(() => {
    if (!card || !rendered) return;
    if (revealed === false) shownAt.current = Date.now();
    if (app.settings.autoplayAudio) playSounds(card.notetype.mediaDir, rendered.sounds);
    else stopAudio();
  }, [rendered, card, revealed, app.settings.autoplayAudio]);

  // 学习中的卡片稍后到期：倒计时后自动刷新
  useEffect(() => {
    if (card || !st?.nextDueAt) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    const wait = Math.max(0, st.nextDueAt * 1000 - Date.now()) + 500;
    const timer = wait < 24 * 3600 * 1000 ? setTimeout(load, wait) : undefined;
    return () => {
      clearInterval(t);
      if (timer) clearTimeout(timer);
    };
  }, [card, st?.nextDueAt, load]);

  const reveal = useCallback(() => {
    if (!card || revealed) return;
    setRevealed(true);
  }, [card, revealed]);

  const answer = useCallback(
    async (rating: number) => {
      if (!card || !revealed || busy.current) return;
      busy.current = true;
      setPressed(rating);
      try {
        const next = await api.answerCard(card.cardId, rating, Date.now() - shownAt.current);
        setSt(next);
        setRevealed(false);
        setTyped("");
        setSessionCount((n) => n + 1);
      } catch (e) {
        toast(String(e), true);
      } finally {
        busy.current = false;
        setTimeout(() => setPressed(null), 120);
      }
    },
    [card, revealed, toast],
  );

  const undo = useCallback(async () => {
    if (busy.current) return;
    try {
      const res = await api.undo();
      if (res) {
        setSt(res);
        setRevealed(false);
        setTyped("");
        setSessionCount((n) => Math.max(0, n - 1));
        toast("已撤销上一次评分");
      }
    } catch (e) {
      toast(String(e), true);
    }
  }, [toast]);

  // 重播：播放这一面上的全部音频；没有音频（或无法播放）时用系统语音朗读单词
  const replay = useCallback(async () => {
    if (!card || !rendered) return;
    if (rendered.allSounds.length) {
      const ok = await playSounds(card.notetype.mediaDir, rendered.allSounds);
      if (ok) return;
      toast("音频文件无法播放，已改用系统朗读");
    }
    if (rendered.tts.length) {
      speak(rendered.tts[0].text, rendered.tts[0].lang || undefined);
      return;
    }
    const word = stripHtml((card.fields[0] ?? "").replace(/\[sound:[^\]]*\]/g, ""));
    if (!speak(word)) toast("这张卡片没有音频", true);
  }, [card, rendered, toast]);

  const markKnown = useCallback(async () => {
    if (!card) return;
    await api.setKnown([card.cardId], true);
    toast("已标为熟词，不会再出现。可在单词本「熟词」中恢复");
    load();
  }, [card, load, toast]);

  const suspend = useCallback(async () => {
    if (!card) return;
    await api.setSuspended([card.cardId], true);
    toast("已暂停这张卡片，可在单词本中恢复");
    load();
  }, [card, load, toast]);

  const onKey = useCallback(
    (e: KeyboardEvent) => {
      if (editing) return;
      const k = e.key;
      if ((e.metaKey || e.ctrlKey) && k.toLowerCase() === "z") {
        e.preventDefault();
        undo();
        return;
      }
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (k === " " || k === "Enter") {
        e.preventDefault();
        if (!revealed) reveal();
        else answer(3);
      } else if (revealed && ["1", "2", "3", "4"].includes(k)) {
        e.preventDefault();
        answer(Number(k));
      } else if (k === "z" || k === "Z") {
        undo();
      } else if (k === "r" || k === "R") {
        replay();
      } else if (k === "k" || k === "K") {
        markKnown();
      } else if (k === "e" || k === "E") {
        if (card) setEditing(true);
      } else if (k === "Escape") {
        app.go({ name: "home" });
      }
    },
    [answer, reveal, revealed, undo, replay, editing, card, app, markKnown],
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

  const counts = st?.counts ?? { new: 0, learning: 0, review: 0 };
  const remaining = counts.new + counts.learning + counts.review;
  const studied = st?.studiedToday ?? 0;
  const pct = studied + remaining > 0 ? (studied / (studied + remaining)) * 100 : 100;
  const activeQueue = card ? (card.state === 0 ? "new" : card.state === 2 ? "review" : "learning") : "";

  return (
    <div className="study">
      <div className="study-top" style={{ paddingLeft: 56 }}>
        <button className="btn btn-ghost" onClick={() => app.go({ name: "home" })}>
          <ArrowLeft size={16} />
          返回
        </button>
        <div className="study-title">{deck?.name ?? card?.deckName ?? ""}</div>
        <div className="study-counts">
          <span className={activeQueue === "new" ? "active" : ""} style={{ color: "var(--new)" }}>
            {counts.new}
            <small>新</small>
          </span>
          <span className={activeQueue === "learning" ? "active" : ""} style={{ color: "var(--learn)" }}>
            {counts.learning}
            <small>学</small>
          </span>
          <span className={activeQueue === "review" ? "active" : ""} style={{ color: "var(--review)" }}>
            {counts.review}
            <small>复</small>
          </span>
        </div>
      </div>
      <div className="study-progress">
        <span style={{ width: `${pct}%` }} />
      </div>

      {st && !card && <Done st={st} sessionCount={sessionCount} now={now} onUndo={undo} />}

      {card && rendered && (
        <div className="study-stage">
          <div className="card-head">
            <span
              className="card-state"
              style={{
                color: STATE_COLOR[card.state],
                background: `color-mix(in srgb, ${STATE_COLOR[card.state]} 12%, transparent)`,
              }}
            >
              {STATE_LABEL[card.state]}
            </span>
            <div className="card-tools">
              <button className="icon-btn" title="重播发音 (R)" onClick={replay}>
                <Volume2 size={16} />
              </button>
              <button className="icon-btn" title="太简单，标为熟词 (K)" onClick={markKnown}>
                <CheckCheck size={16} />
              </button>
              <button className="icon-btn" title="编辑 (E)" onClick={() => setEditing(true)}>
                <Pencil size={15} />
              </button>
              <button className="icon-btn" title="暂停这张卡片" onClick={suspend}>
                <Pause size={15} />
              </button>
              <button className="icon-btn" title="撤销 (Z)" onClick={undo} disabled={!st?.canUndo}>
                <Undo2 size={16} />
              </button>
            </div>
          </div>
          <div className={`card-shell ${revealed ? "flip" : ""}`} key={`${card.cardId}-${revealed ? "a" : "q"}`}>
            <CardFrame
              html={rendered.html}
              css={card.notetype.css}
              mediaDir={card.notetype.mediaDir}
              ord={card.ord}
              dark={app.dark}
              onKey={onKey}
              onEnter={reveal}
              onTyped={setTyped}
              autoFocusInput={!revealed}
            />
          </div>
          <div className="answer-bar">
            {!revealed ? (
              <button className="btn btn-primary show-answer" onClick={reveal}>
                显示答案
                <kbd style={{ background: "rgba(255,255,255,.22)" }}>空格</kbd>
              </button>
            ) : (
              RATINGS.map((b) => (
                <button
                  key={b.r}
                  className={`rate-btn ${b.cls} ${pressed === b.r ? "pressed" : ""}`}
                  onClick={() => answer(b.r)}
                >
                  <kbd>{b.key}</kbd>
                  <span className="label">{b.label}</span>
                  {app.settings.showIntervals && <span className="ivl">{st?.previews[b.r - 1]}</span>}
                </button>
              ))
            )}
          </div>
        </div>
      )}

      {editing && card && (
        <Modal title="编辑卡片" icon={<Pencil size={18} />} onClose={() => setEditing(false)} wide>
          <NoteEditor
            card={card}
            onCancel={() => setEditing(false)}
            onSaved={async () => {
              setEditing(false);
              const fresh = await api.cardDetail(card.cardId);
              setSt((s) => (s ? { ...s, card: fresh.view } : s));
            }}
          />
        </Modal>
      )}
    </div>
  );
}

function Done(props: { st: StudyState; sessionCount: number; now: number; onUndo: () => void }) {
  const app = useApp();
  const { st } = props;
  const waitSecs = st.nextDueAt ? Math.max(0, Math.round(st.nextDueAt - props.now / 1000)) : 0;
  const waitText =
    waitSecs >= 60 ? `${Math.floor(waitSecs / 60)} 分 ${String(waitSecs % 60).padStart(2, "0")} 秒` : `${waitSecs} 秒`;
  return (
    <div className="study-done">
      <div className="done-badge">{st.nextDueAt ? "⏳" : "🎉"}</div>
      <h2>{st.nextDueAt ? "稍事休息" : "太棒了，今天的任务完成了！"}</h2>
      <p>
        {st.nextDueAt
          ? "还有正在学习的单词没到复习时间，到点后会自动出现。"
          : "按照记忆曲线，剩下的单词会在最合适的时候再来找你。明天见～"}
      </p>
      {st.nextDueAt && (
        <div className="waiting">
          <Clock size={16} />
          下一张卡片将在 {waitText} 后到期
        </div>
      )}
      <div className="done-stats">
        <div className="done-stat">
          <b>{props.sessionCount}</b>
          <span>本次学习</span>
        </div>
        <div className="done-stat">
          <b>{st.studiedToday}</b>
          <span>今日累计</span>
        </div>
      </div>
      <div style={{ display: "flex", gap: 10 }}>
        {st.canUndo && (
          <button className="btn" onClick={props.onUndo}>
            <Undo2 size={16} />
            撤销上一张
          </button>
        )}
        <button className="btn" onClick={() => app.go({ name: "stats" })}>
          <BarChart3 size={16} />
          查看统计
        </button>
        <button className="btn btn-primary" onClick={() => app.go({ name: "home" })}>
          <HomeIcon size={16} />
          返回首页
        </button>
      </div>
    </div>
  );
}
