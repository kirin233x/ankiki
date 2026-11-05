import { Monitor, Moon, RefreshCw, Sun } from "lucide-react";
import { Settings } from "../api";
import { useApp } from "../App";
import { Switch } from "../components/ui";

export default function SettingsPage() {
  const { settings, saveSettings, openSync } = useApp();
  const set = <K extends keyof Settings>(k: K, v: Settings[K]) => saveSettings({ ...settings, [k]: v });

  return (
    <div className="page">
      <div className="page-inner" style={{ maxWidth: 760 }}>
        <div className="page-header">
          <div>
            <h1 className="page-title">设置</h1>
            <div className="page-subtitle">调整成你最舒服的学习方式</div>
          </div>
        </div>

        <div className="settings-group">
          <h3>外观</h3>
          <div className="panel">
            <div className="setting-row">
              <div className="setting-text">
                <b>主题</b>
                <span>跟随系统，或固定使用浅色 / 深色</span>
              </div>
              <div className="segmented">
                <button className={settings.theme === "auto" ? "on" : ""} onClick={() => set("theme", "auto")}>
                  <Monitor size={14} />
                  自动
                </button>
                <button className={settings.theme === "light" ? "on" : ""} onClick={() => set("theme", "light")}>
                  <Sun size={14} />
                  浅色
                </button>
                <button className={settings.theme === "dark" ? "on" : ""} onClick={() => set("theme", "dark")}>
                  <Moon size={14} />
                  深色
                </button>
              </div>
            </div>
            <div className="setting-row">
              <div className="setting-text">
                <b>卡片样式</b>
                <span>「原样式」保留词书作者设计的排版；「Ankiki 美化」使用统一的现代字体与配色</span>
              </div>
              <div className="segmented">
                <button
                  className={settings.cardStyle === "original" ? "on" : ""}
                  onClick={() => set("cardStyle", "original")}
                >
                  原样式
                </button>
                <button
                  className={settings.cardStyle === "ankiki" ? "on" : ""}
                  onClick={() => set("cardStyle", "ankiki")}
                >
                  Ankiki 美化
                </button>
              </div>
            </div>
          </div>
        </div>

        <div className="settings-group">
          <h3>学习</h3>
          <div className="panel">
            <div className="setting-row">
              <div className="setting-text">
                <b>自动播放发音</b>
                <span>显示卡片时自动播放其中的音频，按 R 可重播</span>
              </div>
              <Switch on={settings.autoplayAudio} onChange={(v) => set("autoplayAudio", v)} />
            </div>
            <div className="setting-row">
              <div className="setting-text">
                <b>在按钮上显示下次复习时间</b>
                <span>例如「良好 · 3天」</span>
              </div>
              <Switch on={settings.showIntervals} onChange={(v) => set("showIntervals", v)} />
            </div>
            <div className="setting-row">
              <div className="setting-text">
                <b>新的一天开始于</b>
                <span>在这个时间之前学习，仍然算作前一天（适合夜猫子）</span>
              </div>
              <select
                className="select"
                style={{ width: 120 }}
                value={settings.rolloverHour}
                onChange={(e) => set("rolloverHour", Number(e.target.value))}
              >
                {Array.from({ length: 24 }, (_, h) => (
                  <option key={h} value={h}>
                    {String(h).padStart(2, "0")}:00
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>

        <div className="settings-group">
          <h3>Anki</h3>
          <div className="panel">
            <div className="setting-row">
              <div className="setting-text">
                <b>从 Anki 同步学习进度</b>
                <span>
                  读取本机 Anki 的数据，把你在 Anki 里背过的进度和复习记录同步到已导入的同一本词书。
                  可以反复同步，较新的进度会自动保留。
                </span>
              </div>
              <button className="btn" onClick={openSync}>
                <RefreshCw size={15} />
                立即同步
              </button>
            </div>
          </div>
        </div>

        <div className="settings-group">
          <h3>快捷键</h3>
          <div className="panel" style={{ padding: "8px 0" }}>
            {[
              ["空格 / 回车", "显示答案；翻面后等同于「良好」"],
              ["1 2 3 4", "重来 / 困难 / 良好 / 简单"],
              ["Z 或 ⌘Z", "撤销上一次评分"],
              ["R", "重播发音"],
              ["E", "编辑当前卡片"],
              ["Esc", "退出学习"],
            ].map(([k, v]) => (
              <div className="setting-row" key={k} style={{ padding: "10px 20px" }}>
                <div className="setting-text">
                  <span style={{ color: "var(--text)", fontSize: 13.5 }}>{v}</span>
                </div>
                <kbd style={{ fontSize: 12, padding: "3px 8px" }}>{k}</kbd>
              </div>
            ))}
          </div>
        </div>

        <div className="settings-group">
          <h3>关于</h3>
          <div className="panel about">
            <div className="brand-logo">A</div>
            <div className="setting-text">
              <b style={{ fontSize: 16 }}>Ankiki 0.1.0</b>
              <span>
                仿 Anki 的轻量背单词应用 · 使用 FSRS-5 记忆算法 · 兼容 Anki 词书（.apkg / .colpkg）
                <br />
                所有数据都保存在你的电脑本地。
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
