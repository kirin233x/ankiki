import { useEffect, useMemo, useRef } from "react";
import { playSounds, speak } from "../audio";
import { rewriteCssUrls, rewriteMedia } from "../render";

interface Props {
  html: string;
  css: string;
  mediaDir: string;
  ord: number;
  dark: boolean;
  styleMode: "original" | "ankiki";
  /** iframe 内的按键转发给外层（快捷键） */
  onKey?: (e: KeyboardEvent) => void;
  /** 在输入答案框里按回车 */
  onEnter?: () => void;
  /** 点击卡片空白处（手机上用来翻面） */
  onTap?: () => void;
  onTyped?: (value: string) => void;
  autoFocusInput?: boolean;
  compact?: boolean;
}

const BASE_CSS = `
html, body { margin: 0; min-height: 100%; }
html { -webkit-text-size-adjust: 100%; }
body { padding: 44px 40px 56px; box-sizing: border-box; overflow-wrap: break-word; -webkit-font-smoothing: antialiased; }
body.compact { padding: 24px 22px; }
@media (max-width: 600px) { body { padding: 30px 20px 40px; } body.compact { padding: 18px 16px; } }
* { -webkit-tap-highlight-color: transparent; }
img { max-width: 100%; height: auto; border-radius: 8px; }
.replay-button { display: inline-block; width: 38px; height: 38px; vertical-align: middle; margin: 4px 6px; cursor: pointer; transition: transform .15s ease; }
.replay-button:hover { transform: scale(1.08); }
.replay-button:active { transform: scale(.94); }
.replay-button svg { width: 100%; height: 100%; }
.replay-button svg circle { fill: #ffffff; stroke: #d6d3ff; stroke-width: 3; }
.replay-button svg path { fill: #6d5bff; }
.replay-button.tts::before { content: "🔊"; font-size: 24px; line-height: 38px; }
.nightMode .replay-button svg circle { fill: #262a3a; stroke: #4a4f72; }
.nightMode .replay-button svg path { fill: #a99cff; }
.cloze { font-weight: 700; color: #6d5bff; }
.nightMode .cloze { color: #a99cff; }
a.hint { color: #6d5bff; font-size: .85em; text-decoration: none; border-bottom: 1px dashed currentColor; }
input.typeans { font: inherit; font-size: 20px; width: min(420px, 90%); margin: 18px auto 0; display: block; padding: 10px 14px; border-radius: 12px; border: 2px solid #dcd8ff; outline: none; text-align: center; background: rgba(255,255,255,.85); color: inherit; }
input.typeans:focus { border-color: #6d5bff; box-shadow: 0 0 0 4px rgba(109,91,255,.15); }
.nightMode input.typeans { background: rgba(255,255,255,.06); border-color: #3a3f5c; }
.typeans-result { font-family: ui-monospace, Menlo, monospace; font-size: 20px; margin: 14px 0; }
.typeans-good { color: #0e9f6e; font-weight: 600; }
.typeans-bad { color: #e5484d; text-decoration: line-through; }
.typeans-arrow { margin: 0 10px; color: #999; }
::-webkit-scrollbar { width: 8px; }
::-webkit-scrollbar-thumb { background: rgba(127,127,127,.3); border-radius: 4px; }
`;

const ANKIKI_CSS = `
.card {
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif;
  font-size: 22px; line-height: 1.7; text-align: center; color: #1c2030; background: transparent;
}
.card b, .card strong { color: #4b3fd1; }
.card hr, hr#answer { border: none; height: 1px; background: linear-gradient(90deg, transparent, #d9d6f5, transparent); margin: 28px auto; width: 80%; }
.nightMode.card { color: #e8eaf2; }
.nightMode.card b, .nightMode.card strong { color: #b9b0ff; }
.nightMode hr, .nightMode hr#answer { background: linear-gradient(90deg, transparent, #3a3f5c, transparent); }
`;

export default function CardFrame(props: Props) {
  const ref = useRef<HTMLIFrameElement>(null);
  const cb = useRef(props);
  cb.current = props;

  const srcDoc = useMemo(() => {
    const deckCss = props.styleMode === "original" ? rewriteCssUrls(props.css, props.mediaDir) : "";
    const extra = props.styleMode === "ankiki" ? ANKIKI_CSS : "";
    const bodyCls = [
      "card",
      `card${props.ord + 1}`,
      props.dark ? "nightMode night_mode" : "",
      props.compact ? "compact" : "",
    ].join(" ");
    const html = rewriteMedia(props.html, props.mediaDir);
    const transparent = props.styleMode === "ankiki" ? "html,body{background:transparent!important}" : "";
    return `<!doctype html><html><head><meta charset="utf-8"><style>${BASE_CSS}</style><style>${deckCss}</style><style>${extra}${transparent}</style></head><body class="${bodyCls}"><div id="qa">${html}</div></body></html>`;
  }, [props.html, props.css, props.mediaDir, props.ord, props.dark, props.styleMode, props.compact]);

  useEffect(() => {
    const frame = ref.current;
    if (!frame) return;
    const onLoad = () => {
      const doc = frame.contentDocument;
      if (!doc) return;
      doc.addEventListener("click", (e) => {
        const el = e.target as HTMLElement | null;
        const target = el?.closest?.("[data-sound],[data-tts]") as HTMLElement | null;
        if (!target) {
          if (!el?.closest?.("a,button,input,textarea,select,summary,details,audio,video")) cb.current.onTap?.();
          return;
        }
        e.preventDefault();
        if (target.dataset.sound) playSounds(cb.current.mediaDir, [target.dataset.sound]);
        else if (target.dataset.tts) speak(target.dataset.tts, target.dataset.lang);
      });
      doc.addEventListener("keydown", (e) => {
        const t = e.target as HTMLElement;
        const typing = t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable);
        if (typing) {
          if (e.key === "Enter") {
            e.preventDefault();
            cb.current.onEnter?.();
          }
          return;
        }
        cb.current.onKey?.(e);
      });
      const input = doc.getElementById("typeans") as HTMLInputElement | null;
      if (input) {
        input.addEventListener("input", () => cb.current.onTyped?.(input.value));
        if (cb.current.autoFocusInput) setTimeout(() => input.focus(), 30);
      } else {
        // 让外层窗口保持焦点，确保快捷键可用
        if (cb.current.onKey) setTimeout(() => window.focus(), 0);
      }
    };
    frame.addEventListener("load", onLoad);
    return () => frame.removeEventListener("load", onLoad);
  }, []);

  return <iframe ref={ref} className="card-frame" srcDoc={srcDoc} title="card" />;
}
