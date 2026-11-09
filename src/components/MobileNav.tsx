import { useState } from "react";
import { BarChart3, BookOpen, Download, FolderPlus, House, LayoutGrid, Library, Plus, Settings } from "lucide-react";
import type { Route } from "../App";
import { useApp } from "../App";
import { haptic } from "../platform";

const TABS: { key: Route["name"]; label: string; icon: typeof House }[] = [
  { key: "home", label: "首页", icon: House },
  { key: "board", label: "看板", icon: LayoutGrid },
  { key: "browse", label: "单词本", icon: Library },
  { key: "stats", label: "统计", icon: BarChart3 },
  { key: "settings", label: "设置", icon: Settings },
];

/** iOS 风格的悬浮玻璃标签栏 + 独立的「添加」圆形按钮 */
export default function MobileNav() {
  const app = useApp();
  const [sheet, setSheet] = useState(false);
  const due = app.decks.reduce((s, d) => s + d.due.new + d.due.learning + d.due.review, 0);

  return (
    <>
      <div className="m-tabbar-wrap">
        <nav className="m-tabbar glass">
          {TABS.map(({ key, label, icon: Icon }) => {
            const active = app.route.name === key;
            return (
              <button
                key={key}
                className={`m-tab ${active ? "active" : ""}`}
                onClick={() => {
                  haptic();
                  app.go({ name: key } as Route);
                }}
              >
                <span className="m-tab-icon">
                  <Icon size={22} strokeWidth={active ? 2.3 : 1.9} />
                  {key === "home" && due > 0 && <span className="m-dot">{due > 99 ? "99+" : due}</span>}
                </span>
                <span className="m-tab-label">{label}</span>
              </button>
            );
          })}
        </nav>
        <button
          className="m-fab glass"
          aria-label="添加"
          onClick={() => {
            haptic();
            setSheet(true);
          }}
        >
          <Plus size={26} strokeWidth={2.2} />
        </button>
      </div>

      {sheet && (
        <div className="m-sheet-overlay" onClick={() => setSheet(false)}>
          <div className="m-action-sheet" onClick={(e) => e.stopPropagation()}>
            <div className="m-action-group glass">
              <button
                onClick={() => {
                  setSheet(false);
                  app.startImport();
                }}
              >
                <Download size={20} />
                <span>
                  <b>导入 Anki 词书</b>
                  <small>从「文件」中选择 .apkg / .colpkg</small>
                </span>
              </button>
              <button
                disabled={!app.decks.length}
                onClick={() => {
                  setSheet(false);
                  app.addNote();
                }}
              >
                <BookOpen size={20} />
                <span>
                  <b>添加单词</b>
                  <small>手动添加到某本词书</small>
                </span>
              </button>
              <button
                onClick={() => {
                  setSheet(false);
                  app.createDeck();
                }}
              >
                <FolderPlus size={20} />
                <span>
                  <b>新建词书</b>
                  <small>创建一本空白词书</small>
                </span>
              </button>
            </div>
            <button className="m-action-cancel glass" onClick={() => setSheet(false)}>
              取消
            </button>
          </div>
        </div>
      )}
    </>
  );
}
