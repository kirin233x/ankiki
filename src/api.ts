// 与 Rust 后端通信的类型与方法。
// 在浏览器里直接 `npm run dev` 时（没有 Tauri 环境），自动使用 mock 数据，方便调试界面。

export interface DeckConfig {
  newPerDay: number;
  reviewsPerDay: number;
  desiredRetention: number;
  learningSteps: number[];
  relearningSteps: number[];
  maxInterval: number;
  newOrder: number;
  meaningField: string;
  wordField: string;
}

export interface Counts {
  new: number;
  learning: number;
  review: number;
}

export interface DeckInfo {
  id: number;
  name: string;
  description: string;
  createdAt: number;
  config: DeckConfig;
  due: Counts;
  totalCards: number;
  newCards: number;
  learnedCards: number;
  matureCards: number;
  suspendedCards: number;
  knownCards: number;
  studiedToday: number;
}

export interface TemplateDef {
  name: string;
  qfmt: string;
  afmt: string;
}

export interface Notetype {
  id: number;
  name: string;
  kind: number;
  css: string;
  fields: string[];
  templates: TemplateDef[];
  mediaDir: string;
}

export interface CardView {
  cardId: number;
  noteId: number;
  deckId: number;
  deckName: string;
  ord: number;
  state: number;
  position: number;
  fields: string[];
  tags: string;
  notetype: Notetype;
}

export interface StudyState {
  card: CardView | null;
  counts: Counts;
  previews: string[];
  nextDueAt: number | null;
  studiedToday: number;
  canUndo: boolean;
}

export interface ImportProgress {
  stage: string;
  current: number;
  total: number;
}

export interface ImportResult {
  deckId: number;
  deckName: string;
  notesAdded: number;
  cardsAdded: number;
  duplicates: number;
  mediaFiles: number;
  withProgress: number;
  progressUpdated: number;
}

export interface AnkiProfile {
  name: string;
  path: string;
  modified: number;
}

export interface SyncResult {
  matchedNotes: number;
  updatedCards: number;
  skippedNewer: number;
  revlogAdded: number;
  missingNotes: number;
}

export interface Settings {
  theme: "auto" | "light" | "dark";
  rolloverHour: number;
  autoplayAudio: boolean;
  cardStyle: "original" | "ankiki";
  showIntervals: boolean;
}

export interface SearchQuery {
  deckId?: number | null;
  text?: string;
  state?: string;
  sort?: string;
  desc?: boolean;
  offset?: number;
  limit?: number;
}

export interface CardRow {
  cardId: number;
  noteId: number;
  deckId: number;
  deckName: string;
  front: string;
  back: string;
  state: number;
  due: number;
  interval: number;
  stability: number;
  difficulty: number;
  reps: number;
  lapses: number;
  suspended: boolean;
  known: boolean;
  tags: string;
}

export interface SearchResult {
  total: number;
  rows: CardRow[];
}

export interface RevlogEntry {
  ts: number;
  rating: number;
  state: number;
  interval: number;
  durationMs: number;
}

export interface CardDetail {
  view: CardView;
  row: CardRow;
  retrievability: number | null;
  lastReview: number;
  createdAt: number;
  history: RevlogEntry[];
}

export interface DayCount {
  date: string;
  count: number;
}

export interface Stats {
  todayReviews: number;
  todayNew: number;
  todayTimeMs: number;
  todayAgain: number;
  streak: number;
  totalReviews: number;
  totalCards: number;
  newCards: number;
  learningCards: number;
  youngCards: number;
  matureCards: number;
  suspendedCards: number;
  knownCards: number;
  retention30d: number | null;
  avgDaily: number;
  daysStudied: number;
  heatmap: DayCount[];
  forecast: number[];
  ratings30d: number[];
  intervals: number[];
}

export interface BoardQuery {
  deckId?: number | null;
  from?: string | null;
  to?: string | null;
}

export type BoardColor = "red" | "green" | "blue" | "gray";

export interface BoardItem {
  cardId: number;
  noteId: number;
  deckId: number;
  word: string;
  meaning: string;
  color: BoardColor;
  isNew: boolean;
  known: boolean;
  reviews: number;
  worstRating: number;
  interval: number;
  firstTs: number;
  lastTs: number;
}

export interface BoardResult {
  items: BoardItem[];
  newCount: number;
  red: number;
  green: number;
  blue: number;
  gray: number;
  reviews: number;
  today: string;
}

export interface SiftBatch {
  cards: CardView[];
  remaining: number;
  totalNew: number;
  known: number;
}

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

let invokeImpl: Promise<InvokeFn> | null = null;

function getInvoke(): Promise<InvokeFn> {
  if (!invokeImpl) {
    invokeImpl = isTauri
      ? import("@tauri-apps/api/core").then((m) => m.invoke as InvokeFn)
      : import("./mock").then((m) => m.mockInvoke as InvokeFn);
  }
  return invokeImpl;
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const invoke = await getInvoke();
  return invoke<T>(cmd, args);
}

export const api = {
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),

  listDecks: () => call<DeckInfo[]>("list_decks"),
  createDeck: (name: string) => call<number>("create_deck", { name }),
  renameDeck: (deckId: number, name: string) => call<void>("rename_deck", { deckId, name }),
  setDeckDescription: (deckId: number, description: string) =>
    call<void>("set_deck_description", { deckId, description }),
  deleteDeck: (deckId: number) => call<void>("delete_deck", { deckId }),
  updateDeckConfig: (deckId: number, config: DeckConfig) =>
    call<void>("update_deck_config", { deckId, config }),

  studyState: (deckId: number) => call<StudyState>("study_state", { deckId }),
  answerCard: (cardId: number, rating: number, durationMs: number) =>
    call<StudyState>("answer_card", { cardId, rating, durationMs }),
  undo: () => call<StudyState | null>("undo"),

  importPackage: (path: string, options: { includeProgress: boolean; deckName?: string | null }) =>
    call<ImportResult>("import_package", { path, options }),

  findAnkiProfiles: () => call<AnkiProfile[]>("find_anki_profiles"),
  syncFromAnki: (path: string) => call<SyncResult>("sync_from_anki", { path }),

  searchCards: (query: SearchQuery) => call<SearchResult>("search_cards", { query }),
  cardDetail: (cardId: number) => call<CardDetail>("card_detail", { cardId }),
  addNote: (deckId: number, front: string, back: string, tags = "") =>
    call<number>("add_note", { deckId, front, back, tags }),
  updateNote: (noteId: number, fields: string[], tags: string) =>
    call<void>("update_note", { noteId, fields, tags }),
  deleteNotes: (noteIds: number[]) => call<void>("delete_notes", { noteIds }),
  setSuspended: (cardIds: number[], suspended: boolean) =>
    call<void>("set_suspended", { cardIds, suspended }),
  forgetCards: (cardIds: number[]) => call<void>("forget_cards", { cardIds }),
  setKnown: (cardIds: number[], known: boolean) => call<void>("set_known", { cardIds, known }),
  searchCardIds: (query: SearchQuery) => call<number[]>("search_card_ids", { query }),
  deleteCards: (cardIds: number[]) => call<void>("delete_cards", { cardIds }),

  siftBatch: (deckId: number, limit: number, exclude: number[]) =>
    call<SiftBatch>("sift_batch", { deckId, limit, exclude }),
  siftAnswer: (cardId: number, known: boolean) => call<void>("sift_answer", { cardId, known }),
  siftUndo: (cardId: number, position: number) => call<void>("sift_undo", { cardId, position }),

  getStats: (deckId: number | null) => call<Stats>("get_stats", { deckId }),
  wordBoard: (query: BoardQuery) => call<BoardResult>("word_board", { query }),
  deckFields: (deckId: number) => call<string[]>("deck_fields", { deckId }),
};

/** 把本地媒体文件路径转换为 webview 可加载的 URL */
let convertImpl: ((path: string) => string) | null = null;
if (isTauri) {
  import("@tauri-apps/api/core").then((m) => {
    convertImpl = m.convertFileSrc;
  });
}

export function mediaUrl(dir: string, file: string): string {
  if (!isTauri) return `/mock-media/${encodeURIComponent(file)}`;
  const sep = dir.includes("\\") && !dir.includes("/") ? "\\" : "/";
  const path = `${dir}${sep}${file}`;
  if (convertImpl) return convertImpl(path);
  // convertFileSrc 的同步回退实现
  const encoded = encodeURIComponent(path);
  return navigator.userAgent.includes("Windows")
    ? `http://asset.localhost/${encoded}`
    : `asset://localhost/${encoded}`;
}

/** 选择 Anki 词书文件 */
export async function pickPackage(): Promise<string | null> {
  if (!isTauri) return "/mock/四级核心词汇.apkg";
  const { open } = await import("@tauri-apps/plugin-dialog");
  const { isMobile } = await import("./platform");
  // iOS 不认识 .apkg 这种文件类型，加过滤器会导致文件变灰选不了，所以手机上不加过滤
  const res = await open({
    multiple: false,
    directory: false,
    ...(isMobile ? {} : { filters: [{ name: "Anki 词书", extensions: ["apkg", "colpkg"] }] }),
  });
  return typeof res === "string" ? res : null;
}

/** 手动选择 Anki 数据库 collection.anki2 */
export async function pickAnkiCollection(): Promise<string | null> {
  if (!isTauri) return "/mock/collection.anki2";
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({
    multiple: false,
    directory: false,
    title: "选择 Anki 数据库（collection.anki2）",
    filters: [{ name: "Anki 数据库", extensions: ["anki2"] }],
  });
  return typeof res === "string" ? res : null;
}

/** 监听导入进度 */
export async function onImportProgress(cb: (p: ImportProgress) => void): Promise<() => void> {
  if (!isTauri) {
    const { mockOnProgress } = await import("./mock");
    return mockOnProgress(cb);
  }
  const { listen } = await import("@tauri-apps/api/event");
  return listen<ImportProgress>("import-progress", (e) => cb(e.payload));
}

/** 监听把文件拖进窗口 */
export async function onFileDrop(
  cb: (paths: string[]) => void,
  hover: (over: boolean) => void,
): Promise<() => void> {
  if (!isTauri) return () => {};
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  return getCurrentWebview().onDragDropEvent((event) => {
    const p = event.payload;
    if (p.type === "over" || p.type === "enter") hover(true);
    else if (p.type === "leave") hover(false);
    else if (p.type === "drop") {
      hover(false);
      cb(p.paths);
    }
  });
}
