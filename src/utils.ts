export const DECK_GRADIENTS = [
  ["#6d5bff", "#a855f7"],
  ["#ec4899", "#f97316"],
  ["#0ea5e9", "#6366f1"],
  ["#10b981", "#0ea5e9"],
  ["#f59e0b", "#ef4444"],
  ["#8b5cf6", "#ec4899"],
  ["#14b8a6", "#84cc16"],
  ["#f43f5e", "#a855f7"],
];

export function deckGradient(id: number): string {
  const [a, b] = DECK_GRADIENTS[Math.abs(id) % DECK_GRADIENTS.length];
  return `linear-gradient(135deg, ${a} 0%, ${b} 100%)`;
}

export function deckColor(id: number): string {
  return DECK_GRADIENTS[Math.abs(id) % DECK_GRADIENTS.length][0];
}

export function deckGlyph(name: string): string {
  const ch = Array.from(name.trim())[0] ?? "A";
  return /[a-z]/i.test(ch) ? ch.toUpperCase() : ch;
}

export const STATE_LABEL = ["新卡", "学习中", "复习", "重学"];
export const STATE_COLOR = ["var(--new)", "var(--learn)", "var(--review)", "var(--again)"];
export const RATING_LABEL = ["", "重来", "困难", "良好", "简单"];
export const RATING_COLOR = ["", "var(--again)", "var(--hard)", "var(--good)", "var(--easy)"];

export function formatDays(days: number): string {
  if (days <= 0) return "—";
  if (days < 30) return `${days}天`;
  if (days < 365) return `${trim(days / 30)}个月`;
  return `${trim(days / 365)}年`;
}

function trim(v: number): string {
  return v.toFixed(1).replace(/\.0$/, "");
}

export function formatDue(ts: number, state: number): string {
  if (state === 0) return `#${ts + 1}`;
  const now = Date.now() / 1000;
  const diff = ts - now;
  if (state === 1 || state === 3) {
    if (diff <= 0) return "现在";
    if (diff < 3600) return `${Math.ceil(diff / 60)}分钟后`;
    return `${trim(diff / 3600)}小时后`;
  }
  const d = new Date(ts * 1000);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const days = Math.round((new Date(d).setHours(0, 0, 0, 0) - today.getTime()) / 86400000);
  if (days < 0) return `逾期${-days}天`;
  if (days === 0) return "今天";
  if (days === 1) return "明天";
  if (days < 7) return `${days}天后`;
  return `${d.getFullYear() === today.getFullYear() ? "" : d.getFullYear() + "/"}${d.getMonth() + 1}/${d.getDate()}`;
}

export function formatDuration(ms: number): string {
  const mins = Math.round(ms / 60000);
  if (mins < 1) return `${Math.round(ms / 1000)}秒`;
  if (mins < 60) return `${mins}分钟`;
  return `${Math.floor(mins / 60)}小时${mins % 60 ? (mins % 60) + "分" : ""}`;
}

export function formatDateTime(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function greeting(): string {
  const h = new Date().getHours();
  if (h < 5) return "夜深了";
  if (h < 11) return "早上好";
  if (h < 13) return "中午好";
  if (h < 18) return "下午好";
  return "晚上好";
}

export function fileName(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? path;
  try {
    return decodeURIComponent(name);
  } catch {
    return name;
  }
}
