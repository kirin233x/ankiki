// Anki 卡片模板渲染器。
// 支持：{{字段}}、{{FrontSide}}、{{#字段}}…{{/字段}}、{{^字段}}…{{/字段}}、
// 过滤器 text / cloze / hint / type / furigana / kana / kanji / tts，[sound:xxx] 音频，图片相对路径。

import type { CardView } from "./api";
import { mediaUrl } from "./api";

export interface RenderedSide {
  html: string;
  /** 翻到这一面时需要自动播放的音频（答案面不含正面已播放过的） */
  sounds: string[];
  /** 这一面上出现的全部音频（用于「重播」） */
  allSounds: string[];
  /** 这一面上的 TTS 朗读文本 */
  tts: { text: string; lang: string }[];
}

type Node =
  | { t: "text"; v: string }
  | { t: "var"; v: string }
  | { t: "sec"; name: string; inverted: boolean; children: Node[] };

function parse(tpl: string): Node[] {
  const root: Node[] = [];
  const stack: { name: string; nodes: Node[] }[] = [{ name: "", nodes: root }];
  const re = /\{\{([\s\S]*?)\}\}/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(tpl))) {
    const top = stack[stack.length - 1];
    if (m.index > last) top.nodes.push({ t: "text", v: tpl.slice(last, m.index) });
    last = re.lastIndex;
    const tag = m[1].trim();
    if (tag.startsWith("#") || tag.startsWith("^")) {
      const sec: Node = { t: "sec", name: tag.slice(1).trim(), inverted: tag[0] === "^", children: [] };
      top.nodes.push(sec);
      stack.push({ name: sec.name, nodes: sec.children });
    } else if (tag.startsWith("/")) {
      const name = tag.slice(1).trim();
      // 找到匹配的段落并闭合（容忍不规范的嵌套）
      for (let i = stack.length - 1; i > 0; i--) {
        if (stack[i].name === name) {
          stack.length = i;
          break;
        }
      }
    } else if (tag.startsWith("!")) {
      // 注释
    } else {
      top.nodes.push({ t: "var", v: tag });
    }
  }
  if (last < tpl.length) stack[stack.length - 1].nodes.push({ t: "text", v: tpl.slice(last) });
  return root;
}

export function stripHtml(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  return (doc.body.textContent || "").replace(/\s+/g, " ").trim();
}

function fieldEmpty(v: string | undefined): boolean {
  if (!v) return true;
  return v.replace(/<br\s*\/?>|<\/?div>|&nbsp;|\s/gi, "") === "";
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

const CLOZE_RE = /\{\{c(\d+)::([\s\S]*?)(?:::([\s\S]*?))?\}\}/g;

function renderCloze(text: string, ord: number, side: "q" | "a", only = false): string {
  const active = ord + 1;
  if (only) {
    const parts: string[] = [];
    text.replace(CLOZE_RE, (_m, n, body) => {
      if (Number(n) === active) parts.push(body);
      return "";
    });
    return parts.join(", ");
  }
  return text.replace(CLOZE_RE, (_m, n: string, body: string, hint?: string) => {
    if (Number(n) !== active) return body;
    if (side === "q") {
      return `<span class="cloze" data-cloze="${escapeHtml(body)}">[${hint ? escapeHtml(hint) : "..."}]</span>`;
    }
    return `<span class="cloze">${body}</span>`;
  });
}

function furigana(text: string, mode: "furigana" | "kana" | "kanji"): string {
  return text.replace(/ ?([^ >\[]+?)\[(.+?)\]/g, (_m, base: string, reading: string) => {
    if (mode === "kana") return reading;
    if (mode === "kanji") return base;
    return `<ruby><rb>${base}</rb><rt>${reading}</rt></ruby>`;
  });
}

let hintCounter = 0;

export interface RenderContext {
  card: CardView;
  side: "q" | "a";
  frontSide?: string;
  typedAnswer?: string;
}

function applyFilters(filters: string[], fieldName: string, value: string, ctx: RenderContext): string {
  let v = value;
  // Anki 从右往左依次应用过滤器
  for (let i = filters.length - 1; i >= 0; i--) {
    const f = filters[i].trim();
    const fname = f.split(/\s+/)[0];
    switch (fname) {
      case "text":
        v = escapeHtml(stripHtml(v));
        break;
      case "cloze":
        v = renderCloze(v, ctx.card.ord, ctx.side);
        break;
      case "cloze-only":
        v = renderCloze(v, ctx.card.ord, ctx.side, true);
        break;
      case "hint": {
        if (fieldEmpty(v)) break;
        const id = `hint${++hintCounter}`;
        v = `<a class="hint" href="#" onclick="this.style.display='none';document.getElementById('${id}').style.display='block';return false;">显示${escapeHtml(fieldName)}</a><div id="${id}" class="hint" style="display:none">${v}</div>`;
        break;
      }
      case "type": {
        if (ctx.side === "q") {
          v = `<input id="typeans" class="typeans" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" placeholder="输入答案后按回车" />`;
        } else {
          const expected = stripHtml(
            (ctx.card.notetype.kind === 1 ? renderCloze(v, ctx.card.ord, "a", true) : v).replace(SOUND_RE, ""),
          );
          const typed = (ctx.typedAnswer ?? "").trim();
          if (!typed) {
            v = `<div class="typeans-result"><span class="typeans-good">${escapeHtml(expected)}</span></div>`;
          } else if (typed.toLowerCase() === expected.toLowerCase()) {
            v = `<div class="typeans-result"><span class="typeans-good">✓ ${escapeHtml(expected)}</span></div>`;
          } else {
            v = `<div class="typeans-result"><span class="typeans-bad">${escapeHtml(typed)}</span><span class="typeans-arrow">→</span><span class="typeans-good">${escapeHtml(expected)}</span></div>`;
          }
        }
        break;
      }
      case "furigana":
      case "kana":
      case "kanji":
        v = furigana(v, fname);
        break;
      case "tts":
        // 朗读：渲染文本，并附带一个发音按钮
        v = `${v}<a class="replay-button tts" href="#" data-tts="${escapeHtml(stripHtml(v))}" data-lang="${escapeHtml(f.split(/\s+/)[1] ?? "")}"></a>`;
        break;
      default:
        break;
    }
  }
  return v;
}

function renderNodes(nodes: Node[], fields: Record<string, string>, ctx: RenderContext): string {
  let out = "";
  for (const n of nodes) {
    if (n.t === "text") out += n.v;
    else if (n.t === "sec") {
      const val = fields[n.name] ?? specialField(n.name, ctx);
      const empty = n.name.startsWith("c") && /^c\d+$/.test(n.name)
        ? !(ctx.card.ord + 1 === Number(n.name.slice(1)))
        : fieldEmpty(val);
      if (empty === n.inverted) out += renderNodes(n.children, fields, ctx);
    } else {
      const parts = n.v.split(":");
      const name = parts.pop()!.trim();
      const filters = parts;
      if (name === "FrontSide") {
        out += ctx.frontSide ?? "";
        continue;
      }
      const raw = fields[name] ?? specialField(name, ctx);
      if (raw === undefined) continue;
      out += filters.length ? applyFilters(filters, name, raw, ctx) : raw;
    }
  }
  return out;
}

function specialField(name: string, ctx: RenderContext): string | undefined {
  const c = ctx.card;
  switch (name) {
    case "Tags":
      return c.tags;
    case "Deck":
      return c.deckName;
    case "Subdeck":
      return c.deckName.split("::").pop();
    case "Card":
      return c.notetype.templates[c.notetype.kind === 1 ? 0 : c.ord]?.name ?? "";
    case "Type":
      return c.notetype.name;
    case "CardFlag":
      return "";
    default:
      return undefined;
  }
}

const SOUND_RE = /\[sound:([^\]]+)\]/g;

function processSounds(html: string, sounds: string[], autoplay: boolean): string {
  return html.replace(SOUND_RE, (_m, file: string) => {
    const f = file.trim();
    if (autoplay) sounds.push(f);
    return `<a class="replay-button soundLink" href="#" data-sound="${escapeHtml(f)}" aria-label="播放"><svg viewBox="0 0 64 64"><circle cx="32" cy="32" r="29" /><path d="M56.502,32.301l-37.502,20.101l0.329,-40.804l37.173,20.703Z" /></svg></a>`;
  });
}

function collect(html: string) {
  const allSounds: string[] = [];
  const tts: { text: string; lang: string }[] = [];
  html.replace(/data-sound="([^"]*)"/g, (_m, f: string) => {
    allSounds.push(f.replace(/&amp;/g, "&").replace(/&quot;/g, '"').replace(/&#39;/g, "'"));
    return "";
  });
  html.replace(/data-tts="([^"]*)" data-lang="([^"]*)"/g, (_m, t: string, l: string) => {
    tts.push({ text: t, lang: l });
    return "";
  });
  return { allSounds, tts };
}

/** 渲染卡片的一面 */
export function renderSide(card: CardView, side: "q" | "a", typedAnswer?: string): RenderedSide {
  const nt = card.notetype;
  const tpl = nt.kind === 1 ? nt.templates[0] : nt.templates[card.ord] ?? nt.templates[0];
  if (!tpl) return { html: card.fields.join("<br>"), sounds: [], allSounds: [], tts: [] };
  const fields: Record<string, string> = {};
  nt.fields.forEach((name, i) => (fields[name] = card.fields[i] ?? ""));

  const qHtml = renderNodes(parse(tpl.qfmt), fields, { card, side: "q" });
  const sounds: string[] = [];
  if (side === "q") {
    const html = processSounds(qHtml, sounds, true);
    return { html, sounds, ...collect(html) };
  }
  // 答案面：FrontSide 部分的音频不自动重播
  const front = processSounds(qHtml, [], false);
  const aHtml = renderNodes(parse(tpl.afmt), fields, { card, side: "a", frontSide: front, typedAnswer });
  const html = processSounds(aHtml, sounds, true);
  return { html, sounds, ...collect(html) };
}

/** 把相对路径的图片、音视频、CSS url() 改写为本地媒体 URL */
export function rewriteMedia(html: string, mediaDir: string): string {
  const isRelative = (src: string) => !!src && !/^(https?:|data:|blob:|asset:|file:|\/\/|#)/i.test(src);
  const convert = (src: string) => {
    let file = src.replace(/&amp;/g, "&");
    try {
      file = decodeURIComponent(file);
    } catch {
      /* 保持原样 */
    }
    return mediaUrl(mediaDir, file);
  };
  return html.replace(
    /(<(?:img|audio|video|source|embed)\b[^>]*?\ssrc\s*=\s*)(?:"([^"]*)"|'([^']*)'|([^\s>]+))/gi,
    (m, pre: string, dq?: string, sq?: string, bare?: string) => {
      const src = dq ?? sq ?? bare ?? "";
      if (!isRelative(src)) return m;
      return `${pre}"${convert(src).replace(/"/g, "&quot;")}"`;
    },
  );
}

export function rewriteCssUrls(css: string, mediaDir: string): string {
  return css.replace(/url\(\s*(["']?)([^"')]+)\1\s*\)/gi, (m, _q, src: string) => {
    if (/^(https?:|data:|blob:|asset:|file:|\/\/|#)/i.test(src)) return m;
    return `url("${mediaUrl(mediaDir, src)}")`;
  });
}
