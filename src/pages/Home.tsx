import { useEffect, useRef, useState } from "react";
import {
  BarChart3,
  BookOpen,
  Brain,
  Check,
  Clock,
  Download,
  Flame,
  Library,
  MoreHorizontal,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  Settings2,
  Zap,
  Sparkles,
  Trash2,
  Upload,
} from "lucide-react";
import { api, DeckInfo, Stats } from "../api";
import { useApp } from "../App";
import { Confirm, Menu, useToast } from "../components/ui";
import { deckGlyph, deckGradient, formatDuration, greeting } from "../utils";

export default function Home() {
  const app = useApp();
  const { decks, decksLoaded } = app;
  const [stats, setStats] = useState<Stats | null>(null);

  useEffect(() => {
    api.getStats(null).then(setStats).catch(() => {});
  }, [decks]);

  if (decksLoaded && decks.length === 0) return <Empty />;

  const due = decks.reduce(
    (a, d) => ({ new: a.new + d.due.new, learning: a.learning + d.due.learning, review: a.review + d.due.review }),
    { new: 0, learning: 0, review: 0 },
  );
  const total = due.new + due.learning + due.review;
  const firstDue = decks.find((d) => d.due.new + d.due.learning + d.due.review > 0);

  return (
    <div className="page">
      <div className="page-inner">
        <div className="page-header">
          <div>
            <h1 className="page-title">{greeting()} 👋</h1>
            <div className="page-subtitle">
              {total > 0 ? `今天还有 ${total} 张卡片等你，坚持就是胜利。` : "今天的任务都完成啦，好好休息一下吧！"}
            </div>
          </div>
          <div className="header-actions">
            <button className="btn" onClick={app.openSync} title="把 Anki 里背过的进度同步过来">
              <RefreshCw size={16} />
              从 Anki 同步
            </button>
            <button className="btn" onClick={app.createDeck}>
              <Plus size={16} />
              新建词书
            </button>
            <button className="btn" onClick={() => app.addNote()}>
              <BookOpen size={16} />
              添加单词
            </button>
          </div>
        </div>

        <section className="hero">
          <div className="hero-main">
            <div className="hero-label">今日待学</div>
            <div className="hero-number">
              {total}
              <small>张卡片</small>
            </div>
            <div className="hero-split">
              <span>
                <i style={{ background: "#93c5fd" }} />
                新词 {due.new}
              </span>
              <span>
                <i style={{ background: "#fdba74" }} />
                学习中 {due.learning}
              </span>
              <span>
                <i style={{ background: "#6ee7b7" }} />
                待复习 {due.review}
              </span>
            </div>
            <button
              className="btn btn-lg"
              disabled={!firstDue}
              onClick={() => firstDue && app.go({ name: "study", deckId: firstDue.id })}
            >
              {firstDue ? (
                <>
                  <Play size={17} fill="currentColor" />
                  开始学习 · {firstDue.name}
                </>
              ) : (
                <>
                  <Check size={17} />
                  今日已完成
                </>
              )}
            </button>
          </div>
          <div className="hero-stats">
            <div className="hero-stat">
              <div className="hero-stat-num">{stats?.todayReviews ?? 0}</div>
              <div className="hero-stat-label">
                <Brain size={13} />
                今日已学
              </div>
            </div>
            <div className="hero-stat">
              <div className="hero-stat-num">{stats?.streak ?? 0}</div>
              <div className="hero-stat-label">
                <Flame size={13} />
                连续天数
              </div>
            </div>
            <div className="hero-stat">
              <div className="hero-stat-num" style={{ fontSize: 20, lineHeight: "31px" }}>
                {formatDuration(stats?.todayTimeMs ?? 0)}
              </div>
              <div className="hero-stat-label">
                <Clock size={13} />
                今日用时
              </div>
            </div>
          </div>
        </section>

        <div className="section-head">
          <h2 className="section-title">我的词书</h2>
          <button className="btn btn-ghost" onClick={() => app.startImport()}>
            <Download size={15} />
            导入词书
          </button>
        </div>
        <div className="deck-grid">
          {decks.map((d) => (
            <DeckCard key={d.id} deck={d} />
          ))}
          <button className="add-deck-card" onClick={() => app.startImport()}>
            <div className="plus">
              <Upload size={22} />
            </div>
            <b>导入 Anki 词书</b>
            <span style={{ fontSize: 12 }}>支持 .apkg / .colpkg，也可以直接拖进窗口</span>
          </button>
        </div>
      </div>
    </div>
  );
}

function DeckCard({ deck }: { deck: DeckInfo }) {
  const app = useApp();
  const toast = useToast();
  const moreRef = useRef<HTMLButtonElement>(null);
  const [menu, setMenu] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const due = deck.due.new + deck.due.learning + deck.due.review;
  const learnedPct = deck.totalCards ? (deck.learnedCards / deck.totalCards) * 100 : 0;
  const maturePct = deck.totalCards ? (deck.matureCards / deck.totalCards) * 100 : 0;
  const knownPct = deck.totalCards ? (deck.knownCards / deck.totalCards) * 100 : 0;

  return (
    <div className="deck-card">
      <div className="deck-cover" style={{ background: deckGradient(deck.id) }}>
        <span className="deck-tag">{deck.totalCards} 张卡片</span>
        <button ref={moreRef} className="icon-btn" onClick={() => setMenu(true)} aria-label="更多">
          <MoreHorizontal size={18} />
        </button>
        <div className="deck-glyph">{deckGlyph(deck.name)}</div>
      </div>
      <div className="deck-body">
        <div>
          <h3 className="deck-name" title={deck.name}>
            {deck.name}
          </h3>
          <div className="deck-meta">
            {deck.description ||
              `已学 ${Math.round(learnedPct)}% · 已掌握 ${deck.matureCards}${
                deck.knownCards ? ` · 熟词 ${deck.knownCards}` : ""
              } · 每天 ${deck.config.newPerDay} 个新词`}
          </div>
        </div>
        <div
          className="progress"
          title={`已掌握 ${deck.matureCards} · 熟词 ${deck.knownCards} · 已学 ${deck.learnedCards} · 共 ${deck.totalCards}`}
        >
          <span style={{ width: `${knownPct}%`, background: "#a99cff" }} />
          <span style={{ width: `${maturePct}%`, background: "var(--review)" }} />
          <span style={{ width: `${Math.max(0, learnedPct - maturePct - knownPct)}%`, background: "#a7f3d0" }} />
        </div>
        <div className="counts">
          <div className={`count-pill new ${deck.due.new ? "" : "zero"}`}>
            <b>{deck.due.new}</b>
            <span>新词</span>
          </div>
          <div className={`count-pill learn ${deck.due.learning ? "" : "zero"}`}>
            <b>{deck.due.learning}</b>
            <span>学习中</span>
          </div>
          <div className={`count-pill review ${deck.due.review ? "" : "zero"}`}>
            <b>{deck.due.review}</b>
            <span>待复习</span>
          </div>
        </div>
        <div className="deck-actions">
          {due > 0 ? (
            <button className="btn btn-primary" onClick={() => app.go({ name: "study", deckId: deck.id })}>
              <Play size={15} fill="currentColor" />
              开始学习
            </button>
          ) : (
            <div className="done-chip">
              <Sparkles size={15} />
              今日已完成
            </div>
          )}
          <button
            className="btn"
            onClick={() => app.go({ name: "sift", deckId: deck.id })}
            title="快速过词：把认识的单词标为熟词，跳过不学"
            disabled={deck.newCards === 0}
          >
            <Zap size={15} />
            过词
          </button>
          <button className="btn" onClick={() => app.editDeckOptions(deck)} title="学习设置">
            <Settings2 size={16} />
          </button>
        </div>
      </div>
      {menu && moreRef.current && (
        <Menu
          anchor={moreRef.current}
          onClose={() => setMenu(false)}
          items={[
            { label: "快速过词（筛掉熟词）", icon: <Zap size={15} />, onClick: () => app.go({ name: "sift", deckId: deck.id }) },
            { label: "学习设置", icon: <Settings2 size={15} />, onClick: () => app.editDeckOptions(deck) },
            { label: "浏览单词", icon: <Library size={15} />, onClick: () => app.go({ name: "browse", deckId: deck.id }) },
            { label: "查看统计", icon: <BarChart3 size={15} />, onClick: () => app.go({ name: "stats", deckId: deck.id }) },
            { label: "添加单词", icon: <Plus size={15} />, onClick: () => app.addNote(deck.id) },
            { label: "重命名", icon: <Pencil size={15} />, onClick: () => app.renameDeck(deck) },
            "sep",
            { label: "删除词书", icon: <Trash2 size={15} />, danger: true, onClick: () => setConfirmDelete(true) },
          ]}
        />
      )}
      {confirmDelete && (
        <Confirm
          title="删除词书？"
          danger
          confirmText="删除"
          message={
            <>
              「{deck.name}」中的 {deck.totalCards} 张卡片、学习记录和媒体文件都会被永久删除，无法恢复。
            </>
          }
          onClose={() => setConfirmDelete(false)}
          onConfirm={async () => {
            try {
              await api.deleteDeck(deck.id);
              await app.refreshDecks();
              toast("词书已删除");
            } catch (e) {
              toast(String(e), true);
            }
          }}
        />
      )}
    </div>
  );
}

function Empty() {
  const app = useApp();
  return (
    <div className="page">
      <div className="empty">
        <div className="empty-art">
          <div className="c1" />
          <div className="c2" />
          <div className="c3">A</div>
        </div>
        <h2>欢迎来到 Ankiki</h2>
        <p>
          把你在 Anki 里用的词书导入进来，或者新建一本空白词书，
          <br />
          开始轻松、高效地背单词吧。
        </p>
        <div className="empty-actions">
          <button className="btn btn-primary btn-lg" onClick={() => app.startImport()}>
            <Download size={17} />
            导入 Anki 词书
          </button>
          <button className="btn btn-lg" onClick={app.createDeck}>
            <Plus size={17} />
            新建空白词书
          </button>
        </div>
        <div className="empty-tips">
          <div className="tip">
            <div className="tip-icon" style={{ background: "#efedff", color: "#6d5bff" }}>
              <Upload size={17} />
            </div>
            <b>直接拖入</b>
            <span>把 .apkg 文件拖到窗口任意位置即可导入</span>
          </div>
          <div className="tip">
            <div className="tip-icon" style={{ background: "#e0f2fe", color: "#0284c7" }}>
              <Brain size={17} />
            </div>
            <b>FSRS 智能复习</b>
            <span>新一代记忆算法，相同记忆效果下通常比传统 SM-2 复习更少</span>
          </div>
          <div className="tip">
            <div className="tip-icon" style={{ background: "#dcfce7", color: "#16a34a" }}>
              <Sparkles size={17} />
            </div>
            <b>轻巧不卡顿</b>
            <span>原生轻量应用，启动快、不卡顿</span>
          </div>
        </div>
      </div>
    </div>
  );
}
