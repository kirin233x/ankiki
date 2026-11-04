// 浏览器预览用的模拟后端（仅在非 Tauri 环境下加载，不会打包进正式功能逻辑）
import type {
  CardView,
  DeckConfig,
  DeckInfo,
  ImportProgress,
  Notetype,
  SearchQuery,
  Settings,
  Stats,
  StudyState,
} from "./api";

const cfg = (): DeckConfig => ({
  newPerDay: 20,
  reviewsPerDay: 200,
  desiredRetention: 0.9,
  learningSteps: [1, 10],
  relearningSteps: [10],
  maxInterval: 36500,
  newOrder: 0,
});

const vocabNt: Notetype = {
  id: 2,
  name: "四级词汇",
  kind: 0,
  css: `.card{font-family:Georgia,"Songti SC",serif;font-size:21px;text-align:center;color:#222;background:#fffdf8;line-height:1.6}
.w{font-size:46px;font-weight:700;letter-spacing:.5px;margin-top:24px}.p{color:#9a8f7a;font-family:"Lucida Sans Unicode",sans-serif;margin:6px 0 4px}
.m{font-size:23px;margin-top:6px}.e{font-style:italic;color:#6b6f80;margin-top:14px;font-size:18px}.e b{color:#c2410c}
.nightMode.card{background:#1f2230;color:#e6e6ee}.nightMode .e{color:#a5a9bd}`,
  fields: ["Word", "Phonetic", "Meaning", "Example"],
  templates: [
    {
      name: "Card 1",
      qfmt: '<div class="w">{{Word}}</div><div class="p">{{Phonetic}}</div>',
      afmt: '{{FrontSide}}<hr id=answer><div class="m">{{Meaning}}</div>{{#Example}}<div class="e">{{Example}}</div>{{/Example}}',
    },
  ],
  mediaDir: "/mock",
};

const words: [string, string, string, string][] = [
  ["abandon", "/əˈbændən/", "v. 放弃；抛弃", "They had to <b>abandon</b> the car in the snow."],
  ["ability", "/əˈbɪləti/", "n. 能力；才能", "She has the <b>ability</b> to learn quickly."],
  ["absorb", "/əbˈzɔːb/", "v. 吸收；使全神贯注", "Plants <b>absorb</b> carbon dioxide."],
  ["abstract", "/ˈæbstrækt/", "adj. 抽象的 n. 摘要", "Love is an <b>abstract</b> idea."],
  ["academic", "/ˌækəˈdemɪk/", "adj. 学术的；学院的", "Her <b>academic</b> results are excellent."],
  ["accelerate", "/əkˈseləreɪt/", "v. 加速；促进", "The car <b>accelerated</b> smoothly."],
  ["access", "/ˈækses/", "n. 通道；访问 v. 进入", "Students have free <b>access</b> to the library."],
  ["accommodate", "/əˈkɒmədeɪt/", "v. 容纳；适应", "The hotel can <b>accommodate</b> 500 guests."],
  ["accompany", "/əˈkʌmpəni/", "v. 陪伴；伴随", "Children must be <b>accompanied</b> by an adult."],
  ["accomplish", "/əˈkʌmplɪʃ/", "v. 完成；实现", "We <b>accomplished</b> our goal."],
  ["accumulate", "/əˈkjuːmjəleɪt/", "v. 积累；积聚", "Dust <b>accumulates</b> quickly."],
  ["accurate", "/ˈækjərət/", "adj. 准确的；精确的", "The figures are <b>accurate</b>."],
];

interface MCard {
  id: number;
  deckId: number;
  fields: string[];
  state: number;
  due: number;
  interval: number;
  reps: number;
  lapses: number;
  difficulty: number;
  stability: number;
  suspended: boolean;
}

let nextId = 100;
const decks: { id: number; name: string; description: string; config: DeckConfig; createdAt: number }[] = [
  { id: 1, name: "大学英语四级核心词", description: "", config: cfg(), createdAt: 0 },
  { id: 2, name: "雅思 7 分必备", description: "", config: cfg(), createdAt: 0 },
  { id: 3, name: "日语 N3 单词", description: "", config: { ...cfg(), newPerDay: 15 }, createdAt: 0 },
];
const now = () => Math.floor(Date.now() / 1000);
const cards: MCard[] = [];
words.forEach((w, i) =>
  cards.push({
    id: nextId++,
    deckId: 1,
    fields: [...w],
    state: i < 4 ? 2 : 0,
    due: i < 4 ? now() - 3600 : i,
    interval: i < 4 ? 3 + i * 4 : 0,
    reps: i < 4 ? 4 + i : 0,
    lapses: i === 2 ? 1 : 0,
    difficulty: i < 4 ? 3 + i * 1.5 : 0,
    stability: i < 4 ? 4 + i * 5 : 0,
    suspended: false,
  }),
);
for (let i = 0; i < 40; i++)
  cards.push({
    id: nextId++,
    deckId: 2,
    fields: [`word${i}`, "", "释义", ""],
    state: i < 25 ? 2 : 0,
    due: i < 25 ? now() + (i % 6) * 86400 : i,
    interval: 10 + i,
    reps: 6,
    lapses: 0,
    difficulty: 5,
    stability: 20,
    suspended: false,
  });
let studied = 7;
let settings: Settings = { theme: "auto", rolloverHour: 4, autoplayAudio: true, cardStyle: "original", showIntervals: true };
let lastUndo: MCard | null = null;

function view(c: MCard): CardView {
  const d = decks.find((x) => x.id === c.deckId)!;
  return { cardId: c.id, noteId: c.id, deckId: d.id, deckName: d.name, ord: 0, state: c.state, fields: c.fields, tags: "", notetype: vocabNt };
}

function counts(deckId: number) {
  const cs = cards.filter((c) => c.deckId === deckId && !c.suspended);
  const d = decks.find((x) => x.id === deckId)!;
  return {
    new: Math.min(d.config.newPerDay, cs.filter((c) => c.state === 0).length),
    learning: cs.filter((c) => (c.state === 1 || c.state === 3) && c.due < now() + 86400).length,
    review: cs.filter((c) => c.state === 2 && c.due <= now()).length,
  };
}

function studyState(deckId: number): StudyState {
  const cs = cards.filter((c) => c.deckId === deckId && !c.suspended);
  const ct = counts(deckId);
  const card =
    cs.find((c) => (c.state === 1 || c.state === 3) && c.due <= now()) ??
    cs.find((c) => c.state === 2 && c.due <= now()) ??
    (ct.new > 0 ? cs.find((c) => c.state === 0) : undefined) ??
    cs.find((c) => (c.state === 1 || c.state === 3) && c.due <= now() + 1200);
  const later = cs.filter((c) => c.state === 1 || c.state === 3).map((c) => c.due);
  return {
    card: card ? view(card) : null,
    counts: ct,
    previews: card?.state === 2 ? ["10分钟", `${card.interval}天`, `${card.interval * 2 + 1}天`, `${card.interval * 3 + 2}天`] : ["1分钟", "6分钟", "10分钟", "3天"],
    nextDueAt: card ? null : later.length ? Math.min(...later) : null,
    studiedToday: studied,
    canUndo: !!lastUndo,
  };
}

function deckInfo(): DeckInfo[] {
  return decks.map((d) => {
    const cs = cards.filter((c) => c.deckId === d.id);
    const total = d.id === 3 ? 0 : cs.length;
    return {
      ...d,
      due: d.id === 3 ? { new: 0, learning: 0, review: 0 } : counts(d.id),
      totalCards: total,
      newCards: cs.filter((c) => c.state === 0).length,
      learnedCards: cs.filter((c) => c.state !== 0).length,
      matureCards: cs.filter((c) => c.state === 2 && c.interval >= 21).length,
      suspendedCards: cs.filter((c) => c.suspended).length,
      studiedToday: 0,
    };
  });
}

function stats(): Stats {
  const heatmap = Array.from({ length: 371 }, (_, i) => {
    const d = new Date(Date.now() - (370 - i) * 86400000);
    const seed = Math.sin(i * 12.9898) * 43758.5453;
    const r = seed - Math.floor(seed);
    return { date: d.toISOString().slice(0, 10), count: i > 200 && r > 0.25 ? Math.round(r * 120) : 0 };
  });
  return {
    todayReviews: studied,
    todayNew: 3,
    todayTimeMs: 6.5 * 60000,
    todayAgain: 1,
    streak: 23,
    totalReviews: 8421,
    totalCards: cards.length,
    newCards: cards.filter((c) => c.state === 0).length,
    learningCards: 2,
    youngCards: 18,
    matureCards: 11,
    suspendedCards: 1,
    retention30d: 0.893,
    avgDaily: 64.2,
    daysStudied: 131,
    heatmap,
    forecast: Array.from({ length: 30 }, (_, i) => Math.round(20 + 15 * Math.sin(i / 3) + (i === 0 ? 20 : 0))),
    ratings30d: [92, 140, 1210, 260],
    intervals: [4, 12, 18, 25, 30, 22, 9, 3],
  };
}

const listeners: ((p: ImportProgress) => void)[] = [];
export function mockOnProgress(cb: (p: ImportProgress) => void) {
  listeners.push(cb);
  return () => listeners.splice(listeners.indexOf(cb), 1);
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function mockInvoke<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
  await sleep(30);
  const r = (v: unknown) => v as T;
  switch (cmd) {
    case "get_settings":
      return r(settings);
    case "save_settings":
      settings = args.settings;
      return r(null);
    case "list_decks":
      return r(deckInfo());
    case "create_deck":
      decks.push({ id: ++nextId, name: args.name, description: "", config: cfg(), createdAt: now() });
      return r(nextId);
    case "rename_deck":
      decks.find((d) => d.id === args.deckId)!.name = args.name;
      return r(null);
    case "delete_deck":
      decks.splice(decks.findIndex((d) => d.id === args.deckId), 1);
      return r(null);
    case "update_deck_config":
      decks.find((d) => d.id === args.deckId)!.config = args.config;
      return r(null);
    case "set_deck_description":
      decks.find((d) => d.id === args.deckId)!.description = args.description;
      return r(null);
    case "study_state":
      return r(studyState(args.deckId));
    case "answer_card": {
      const c = cards.find((x) => x.id === args.cardId)!;
      lastUndo = { ...c };
      studied++;
      if (args.rating === 1) {
        c.state = c.state === 2 ? 3 : 1;
        c.due = now() + 60;
      } else {
        c.state = 2;
        c.interval = Math.max(1, c.interval * 2);
        c.due = now() + c.interval * 86400;
      }
      c.reps++;
      return r(studyState(c.deckId));
    }
    case "undo": {
      if (!lastUndo) return r(null);
      const i = cards.findIndex((x) => x.id === lastUndo!.id);
      cards[i] = lastUndo;
      lastUndo = null;
      studied--;
      const st = studyState(cards[i].deckId);
      st.card = view(cards[i]);
      return r(st);
    }
    case "import_package": {
      for (let i = 0; i <= 10; i++) {
        listeners.forEach((l) => l({ stage: i < 6 ? "正在导入单词…" : "正在复制音频和图片…", current: i * 100, total: 1000 }));
        await sleep(120);
      }
      const id = ++nextId;
      decks.push({ id, name: "四级核心词汇", description: "", config: cfg(), createdAt: now() });
      words.forEach((w, i) =>
        cards.push({ id: nextId++, deckId: id, fields: [...w], state: 0, due: i, interval: 0, reps: 0, lapses: 0, difficulty: 0, stability: 0, suspended: false }),
      );
      return r({ deckId: id, deckName: "四级核心词汇", notesAdded: 1254, cardsAdded: 1254, duplicates: 0, mediaFiles: 1254, withProgress: 0 });
    }
    case "search_cards": {
      const q: SearchQuery = args.query;
      let rs = cards.filter((c) => (q.deckId ? c.deckId === q.deckId : true));
      if (q.text) rs = rs.filter((c) => c.fields.join(" ").includes(q.text!));
      if (q.state === "new") rs = rs.filter((c) => c.state === 0);
      if (q.state === "review") rs = rs.filter((c) => c.state === 2);
      if (q.state === "suspended") rs = rs.filter((c) => c.suspended);
      return r({
        total: rs.length,
        rows: rs.slice(q.offset ?? 0, (q.offset ?? 0) + (q.limit ?? 50)).map((c) => ({
          cardId: c.id,
          noteId: c.id,
          deckId: c.deckId,
          deckName: decks.find((d) => d.id === c.deckId)?.name ?? "",
          front: c.fields[0],
          back: c.fields[2].replace(/<[^>]+>/g, ""),
          state: c.state,
          due: c.due,
          interval: c.interval,
          stability: c.stability,
          difficulty: c.difficulty,
          reps: c.reps,
          lapses: c.lapses,
          suspended: c.suspended,
          tags: "",
        })),
      });
    }
    case "card_detail": {
      const c = cards.find((x) => x.id === args.cardId)!;
      return r({
        view: view(c),
        row: { cardId: c.id, noteId: c.id, deckId: c.deckId, deckName: "", front: c.fields[0], back: c.fields[2], state: c.state, due: c.due, interval: c.interval, stability: c.stability, difficulty: c.difficulty, reps: c.reps, lapses: c.lapses, suspended: c.suspended, tags: "" },
        retrievability: c.state ? 0.87 : null,
        lastReview: now() - 86400 * 3,
        createdAt: now() - 86400 * 30,
        history: c.state
          ? [
              { ts: Date.now() - 86400000 * 3, rating: 3, state: 2, interval: c.interval, durationMs: 4200 },
              { ts: Date.now() - 86400000 * 9, rating: 1, state: 2, interval: -600, durationMs: 9100 },
              { ts: Date.now() - 86400000 * 20, rating: 3, state: 0, interval: -600, durationMs: 5600 },
            ]
          : [],
      });
    }
    case "set_suspended":
      for (const id of args.cardIds) cards.find((c) => c.id === id)!.suspended = args.suspended;
      return r(null);
    case "get_stats":
      return r(stats());
    default:
      return r(null);
  }
}
