import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { BarChart3, BookOpen, Download, House, Library, Plus, Settings as SettingsIcon, LayoutGrid } from "lucide-react";
import { api, DeckInfo, onFileDrop, pickPackage, Settings, Stats } from "./api";
import { ToastProvider, useToast } from "./components/ui";
import ImportDialog from "./components/ImportDialog";
import AddNoteDialog from "./components/AddNoteDialog";
import DeckOptionsDialog from "./components/DeckOptionsDialog";
import PromptDialog from "./components/PromptDialog";
import SyncDialog from "./components/SyncDialog";
import Home from "./pages/Home";
import Study from "./pages/Study";
import Sift from "./pages/Sift";
import Browse from "./pages/Browse";
import Board from "./pages/Board";
import StatsPage from "./pages/Stats";
import SettingsPage from "./pages/Settings";
import { deckColor } from "./utils";
import { isMobile } from "./platform";
import MobileNav from "./components/MobileNav";

export type Route =
  | { name: "home" }
  | { name: "study"; deckId: number }
  | { name: "sift"; deckId: number }
  | { name: "browse"; deckId?: number | null }
  | { name: "stats"; deckId?: number | null }
  | { name: "board"; date?: string }
  | { name: "settings" };

interface AppCtx {
  settings: Settings;
  saveSettings: (s: Settings) => Promise<void>;
  decks: DeckInfo[];
  decksLoaded: boolean;
  /** 首页用的全局统计（与词书一起加载，避免首页先空后满地闪一下） */
  homeStats: Stats | null;
  refreshDecks: () => Promise<void>;
  route: Route;
  go: (r: Route) => void;
  dark: boolean;
  startImport: (path?: string) => void;
  addNote: (deckId?: number) => void;
  editDeckOptions: (deck: DeckInfo) => void;
  createDeck: () => void;
  renameDeck: (deck: DeckInfo) => void;
  openSync: () => void;
}

const Ctx = createContext<AppCtx>(null as unknown as AppCtx);
export const useApp = () => useContext(Ctx);

const DEFAULT_SETTINGS: Settings = {
  theme: "auto",
  rolloverHour: 4,
  autoplayAudio: true,
  cardStyle: "original",
  showIntervals: true,
};

function useSystemDark() {
  const [dark, setDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const fn = () => setDark(mq.matches);
    mq.addEventListener("change", fn);
    return () => mq.removeEventListener("change", fn);
  }, []);
  return dark;
}

/** 数据准备好、首屏已经画出来之后，淡出启动画面 */
function hideSplash() {
  const el = document.getElementById("splash");
  if (!el || el.classList.contains("hide")) return;
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      el.classList.add("hide");
      setTimeout(() => el.remove(), 400);
    }),
  );
}

function Shell() {
  const toast = useToast();
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [decks, setDecks] = useState<DeckInfo[]>([]);
  const [decksLoaded, setDecksLoaded] = useState(false);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [homeStats, setHomeStats] = useState<Stats | null>(null);
  const [route, setRoute] = useState<Route>({ name: "home" });
  const [importPath, setImportPath] = useState<string | null>(null);
  const [addNoteDeck, setAddNoteDeck] = useState<number | null | undefined>(undefined);
  const [optionsDeck, setOptionsDeck] = useState<DeckInfo | null>(null);
  const [prompt, setPrompt] = useState<null | { kind: "create" } | { kind: "rename"; deck: DeckInfo }>(null);
  const [dropping, setDropping] = useState(false);
  const [syncOpen, setSyncOpen] = useState(false);
  const systemDark = useSystemDark();
  const dark = settings.theme === "dark" || (settings.theme === "auto" && systemDark);

  useEffect(() => {
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  }, [dark]);

  const refreshDecks = useCallback(async () => {
    try {
      const [d, st] = await Promise.all([api.listDecks(), api.getStats(null).catch(() => null)]);
      setDecks(d);
      if (st) setHomeStats(st);
    } catch (e) {
      toast(String(e), true);
    } finally {
      setDecksLoaded(true);
    }
  }, [toast]);

  useEffect(() => {
    api
      .getSettings()
      .then(setSettings)
      .catch(() => {})
      .finally(() => setSettingsLoaded(true));
    refreshDecks();
  }, [refreshDecks]);

  const ready = settingsLoaded && decksLoaded;
  useEffect(() => {
    if (ready) hideSplash();
  }, [ready]);
  useEffect(() => {
    // 兜底：无论如何 8 秒后都进入 App
    const t = setTimeout(hideSplash, 8000);
    return () => clearTimeout(t);
  }, []);

  // 跨过学习日或回到窗口时刷新到期数量
  useEffect(() => {
    const fn = () => document.visibilityState === "visible" && refreshDecks();
    document.addEventListener("visibilitychange", fn);
    const timer = setInterval(refreshDecks, 5 * 60 * 1000);
    return () => {
      document.removeEventListener("visibilitychange", fn);
      clearInterval(timer);
    };
  }, [refreshDecks]);

  const startImport = useCallback(
    async (path?: string) => {
      const p = path ?? (await pickPackage());
      if (p) setImportPath(p);
    },
    [],
  );

  useEffect(() => {
    let off: (() => void) | undefined;
    onFileDrop(
      (paths) => {
        const pkg = paths.find((p) => /\.(apkg|colpkg)$/i.test(p));
        if (pkg) startImport(pkg);
        else toast("请拖入 .apkg 或 .colpkg 格式的 Anki 词书", true);
      },
      setDropping,
    ).then((f) => (off = f));
    return () => off?.();
  }, [startImport, toast]);

  const saveSettings = useCallback(
    async (s: Settings) => {
      setSettings(s);
      try {
        await api.saveSettings(s);
      } catch (e) {
        toast(String(e), true);
      }
    },
    [toast],
  );

  const go = useCallback((r: Route) => setRoute(r), []);

  const ctx = useMemo<AppCtx>(
    () => ({
      settings,
      saveSettings,
      decks,
      decksLoaded,
      homeStats,
      refreshDecks,
      route,
      go,
      dark,
      startImport,
      addNote: (deckId?: number) => setAddNoteDeck(deckId ?? null),
      editDeckOptions: setOptionsDeck,
      createDeck: () => setPrompt({ kind: "create" }),
      renameDeck: (deck) => setPrompt({ kind: "rename", deck }),
      openSync: () => setSyncOpen(true),
    }),
    [settings, saveSettings, decks, decksLoaded, homeStats, refreshDecks, route, go, dark, startImport],
  );

  const totalDue = decks.reduce((s, d) => s + d.due.new + d.due.learning + d.due.review, 0);
  const studying = route.name === "study" || route.name === "sift";

  const nav = [
    { key: "home", label: "首页", icon: <House size={18} />, route: { name: "home" } as Route, badge: totalDue },
    { key: "browse", label: "单词本", icon: <Library size={18} />, route: { name: "browse" } as Route },
    { key: "board", label: "词表看板", icon: <LayoutGrid size={18} />, route: { name: "board" } as Route },
    { key: "stats", label: "学习统计", icon: <BarChart3 size={18} />, route: { name: "stats" } as Route },
    { key: "settings", label: "设置", icon: <SettingsIcon size={18} />, route: { name: "settings" } as Route },
  ];

  // 数据加载完成前只渲染空的底色（手机上被启动画面盖住），避免先出现空页面再闪成有数据的页面
  if (!ready) return <div className="app" />;

  return (
    <Ctx.Provider value={ctx}>
      <div className={`app ${studying ? "studying" : ""}`}>
        {isMobile && <div className="m-backdrop" aria-hidden />}
        {!studying && !isMobile && (
          <aside className="sidebar">
            <div className="drag-region" data-tauri-drag-region />
            <div className="brand" data-tauri-drag-region>
              <div className="brand-logo">A</div>
              <div>
                <div className="brand-name">Ankiki</div>
                <div className="brand-sub">轻量 · 专注 · 好记</div>
              </div>
            </div>
            <nav className="nav">
              {nav.map((n) => (
                <button
                  key={n.key}
                  className={`nav-item ${route.name === n.key ? "active" : ""}`}
                  onClick={() => go(n.route)}
                >
                  {n.icon}
                  {n.label}
                  {!!n.badge && <span className="badge badge-accent">{n.badge}</span>}
                </button>
              ))}
            </nav>
            <div className="sidebar-section">我的词书</div>
            <div className="sidebar-decks">
              {decks.map((d) => {
                const due = d.due.new + d.due.learning + d.due.review;
                return (
                  <button key={d.id} className="sidebar-deck" onClick={() => go({ name: "study", deckId: d.id })}>
                    <span className="dot" style={{ background: deckColor(d.id) }} />
                    <span className="name">{d.name}</span>
                    <span className="num">{due || ""}</span>
                  </button>
                );
              })}
              <button className="sidebar-deck" style={{ color: "var(--muted)" }} onClick={() => setPrompt({ kind: "create" })}>
                <Plus size={14} />
                <span className="name">新建词书</span>
              </button>
            </div>
            <div className="sidebar-footer">
              <button className="btn btn-soft btn-block" onClick={() => setAddNoteDeck(null)} disabled={!decks.length}>
                <BookOpen size={16} />
                添加单词
              </button>
              <button className="btn btn-primary btn-block" onClick={() => startImport()}>
                <Download size={16} />
                导入 Anki 词书
              </button>
            </div>
          </aside>
        )}
        <main className="main">
          {!isMobile && <div className="main-drag" data-tauri-drag-region />}
          {route.name === "home" && <Home />}
          {route.name === "study" && <Study key={route.deckId} deckId={route.deckId} />}
          {route.name === "sift" && <Sift key={route.deckId} deckId={route.deckId} />}
          {route.name === "browse" && <Browse initialDeck={route.deckId ?? null} />}
          {route.name === "stats" && <StatsPage initialDeck={route.deckId ?? null} />}
          {route.name === "board" && <Board key={route.date ?? ""} initialDate={route.date} />}
          {route.name === "settings" && <SettingsPage />}
        </main>
        {isMobile && !studying && <MobileNav />}
      </div>

      {importPath && (
        <ImportDialog
          path={importPath}
          onClose={() => setImportPath(null)}
          onDone={() => refreshDecks()}
        />
      )}
      {addNoteDeck !== undefined && (
        <AddNoteDialog initialDeck={addNoteDeck} onClose={() => setAddNoteDeck(undefined)} />
      )}
      {optionsDeck && <DeckOptionsDialog deck={optionsDeck} onClose={() => setOptionsDeck(null)} />}
      {prompt && (
        <PromptDialog
          title={prompt.kind === "create" ? "新建词书" : "重命名词书"}
          label="词书名称"
          placeholder="例如：考研英语核心词"
          initial={prompt.kind === "rename" ? prompt.deck.name : ""}
          confirmText={prompt.kind === "create" ? "创建" : "保存"}
          onClose={() => setPrompt(null)}
          onSubmit={async (name) => {
            if (prompt.kind === "create") {
              await api.createDeck(name);
              toast(`已创建「${name}」`);
            } else {
              await api.renameDeck(prompt.deck.id, name);
            }
            await refreshDecks();
          }}
        />
      )}
      {syncOpen && <SyncDialog onClose={() => setSyncOpen(false)} onDone={() => refreshDecks()} />}
      {dropping && <div className="drop-overlay">松开鼠标，导入 Anki 词书</div>}
    </Ctx.Provider>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <Shell />
    </ToastProvider>
  );
}
