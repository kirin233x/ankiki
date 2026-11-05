import { useEffect, useMemo, useState } from "react";
import { Brain, CalendarCheck, Clock, Flame, Target } from "lucide-react";
import { api, Stats } from "../api";
import { useApp } from "../App";
import { formatDuration } from "../utils";

export default function StatsPage({ initialDeck }: { initialDeck: number | null }) {
  const app = useApp();
  const [deckId, setDeckId] = useState<number | null>(initialDeck);
  const [s, setS] = useState<Stats | null>(null);

  useEffect(() => {
    api.getStats(deckId).then(setS).catch(() => {});
  }, [deckId]);

  return (
    <div className="page">
      <div className="page-inner">
        <div className="page-header">
          <div>
            <h1 className="page-title">学习统计</h1>
            <div className="page-subtitle">
              {s ? `累计复习 ${s.totalReviews} 次 · 学习了 ${s.daysStudied} 天 · 平均每天 ${Math.round(s.avgDaily)} 次` : "加载中…"}
            </div>
          </div>
          <select
            className="select"
            style={{ width: 200 }}
            value={deckId ?? ""}
            onChange={(e) => setDeckId(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">全部词书</option>
            {app.decks.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>
        </div>

        {s && (
          <>
            <div className="kpi-grid">
              <Kpi icon={<Brain size={18} />} color="#6d5bff" num={s.todayReviews} unit="次" label="今日复习" />
              <Kpi icon={<CalendarCheck size={18} />} color="#3b82f6" num={s.todayNew} unit="个" label="今日新学" />
              <Kpi icon={<Clock size={18} />} color="#0ea5e9" num={formatDuration(s.todayTimeMs)} label="今日用时" />
              <Kpi icon={<Flame size={18} />} color="#f97316" num={s.streak} unit="天" label="连续学习" />
              <Kpi
                icon={<Target size={18} />}
                color="#10b981"
                num={s.retention30d != null ? Math.round(s.retention30d * 100) : "—"}
                unit={s.retention30d != null ? "%" : ""}
                label="近 30 天记忆率"
              />
            </div>

            <div className="panel panel-pad">
              <div className="panel-title">学习日历</div>
              <div className="panel-sub">过去一年每天的复习次数</div>
              <Heatmap data={s.heatmap} />
            </div>

            <div className="stats-grid">
              <div className="panel panel-pad">
                <div className="panel-title">未来 30 天复习预测</div>
                <div className="panel-sub">每天将要到期的卡片数量</div>
                <Bars
                  values={s.forecast}
                  labels={s.forecast.map((_, i) => (i === 0 ? "今天" : i % 5 === 0 ? `${i}天` : ""))}
                />
              </div>
              <div className="panel panel-pad">
                <div className="panel-title">卡片分布</div>
                <div className="panel-sub">共 {s.totalCards} 张卡片 · 间隔 ≥ 21 天视为已掌握</div>
                <Donut
                  parts={[
                    { label: "新卡", value: s.newCards, color: "#3b82f6" },
                    { label: "学习中", value: s.learningCards, color: "#f97316" },
                    { label: "复习中", value: s.youngCards, color: "#6ee7b7" },
                    { label: "已掌握", value: s.matureCards, color: "#10b981" },
                    { label: "熟词", value: s.knownCards, color: "#a99cff" },
                    { label: "已暂停", value: s.suspendedCards, color: "#c4c8d6" },
                  ]}
                />
              </div>
              <div className="panel panel-pad">
                <div className="panel-title">复习间隔分布</div>
                <div className="panel-sub">间隔越长，说明记得越牢</div>
                <Bars
                  values={s.intervals}
                  labels={["<1天", "1-2天", "3-6天", "1-2周", "2-4周", "1-3月", "3-6月", "半年+"]}
                />
              </div>
              <div className="panel panel-pad">
                <div className="panel-title">近 30 天评分</div>
                <div className="panel-sub">你按下每个按钮的次数</div>
                <Ratings values={s.ratings30d} />
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function Kpi(props: { icon: React.ReactNode; color: string; num: number | string; unit?: string; label: string }) {
  return (
    <div className="kpi">
      <div className="kpi-icon" style={{ background: `${props.color}1a`, color: props.color }}>
        {props.icon}
      </div>
      <div className="kpi-num">
        {props.num}
        {props.unit && <small>{props.unit}</small>}
      </div>
      <div className="kpi-label">{props.label}</div>
    </div>
  );
}

const HEAT = ["var(--panel-3)", "#d9d3ff", "#b3a8ff", "#8b7bff", "#6d5bff"];

function Heatmap({ data }: { data: { date: string; count: number }[] }) {
  const { weeks, months, max } = useMemo(() => {
    // 第一列从周日开始对齐
    const first = data.length ? new Date(data[0].date + "T00:00:00") : new Date();
    const pad = first.getDay();
    const cells: ({ date: string; count: number } | null)[] = [...Array(pad).fill(null), ...data];
    const weeks: (typeof cells)[] = [];
    for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
    const months: string[] = weeks.map((w, i) => {
      const d = w.find(Boolean);
      if (!d) return "";
      const m = Number(d.date.slice(5, 7));
      const prev = weeks[i - 1]?.find(Boolean);
      return prev && Number(prev.date.slice(5, 7)) !== m ? `${m}月` : "";
    });
    const max = Math.max(1, ...data.map((d) => d.count));
    return { weeks, months, max };
  }, [data]);

  const level = (c: number) => (c === 0 ? 0 : Math.min(4, Math.ceil((c / max) * 4)));

  return (
    <div>
      <div style={{ overflowX: "auto" }}>
        <div className="heatmap-months">
          {months.map((m, i) => (
            <div key={i} style={{ width: 13, flexShrink: 0, whiteSpace: "nowrap" }}>
              {m}
            </div>
          ))}
        </div>
        <div className="heatmap">
          {weeks.map((w, i) => (
            <div className="heatmap-col" key={i}>
              {w.map((d, j) => (
                <div
                  key={j}
                  className="heatmap-cell"
                  title={d ? `${d.date}：${d.count} 次` : ""}
                  style={{ background: d ? HEAT[level(d.count)] : "transparent" }}
                />
              ))}
            </div>
          ))}
        </div>
      </div>
      <div className="heatmap-legend">
        少
        {HEAT.map((c, i) => (
          <div key={i} className="heatmap-cell" style={{ background: c, width: 11, height: 11 }} />
        ))}
        多
      </div>
    </div>
  );
}

function Bars({ values, labels }: { values: number[]; labels: string[] }) {
  const max = Math.max(1, ...values);
  return (
    <div className="bars">
      {values.map((v, i) => (
        <div className="bar" key={i}>
          <span className="tip-num">{v}</span>
          <span style={{ height: `${(v / max) * 100}%`, opacity: v ? 1 : 0.25 }} />
          <em>{labels[i] || " "}</em>
        </div>
      ))}
    </div>
  );
}

function Donut({ parts }: { parts: { label: string; value: number; color: string }[] }) {
  const total = parts.reduce((s, p) => s + p.value, 0);
  const R = 58;
  const C = 2 * Math.PI * R;
  let acc = 0;
  return (
    <div className="donut-wrap">
      <svg width="150" height="150" viewBox="0 0 150 150" style={{ flexShrink: 0 }}>
        <circle cx="75" cy="75" r={R} fill="none" stroke="var(--panel-3)" strokeWidth="18" />
        {total > 0 &&
          parts.map((p, i) => {
            const len = (p.value / total) * C;
            const el = (
              <circle
                key={i}
                cx="75"
                cy="75"
                r={R}
                fill="none"
                stroke={p.color}
                strokeWidth="18"
                strokeDasharray={`${len} ${C - len}`}
                strokeDashoffset={-acc}
                transform="rotate(-90 75 75)"
              />
            );
            acc += len;
            return el;
          })}
        <text x="75" y="72" textAnchor="middle" className="donut-center" fill="var(--text)">
          {total > 0 ? Math.round((((parts[3]?.value ?? 0) + (parts[4]?.value ?? 0)) / total) * 100) : 0}%
        </text>
        <text x="75" y="92" textAnchor="middle" fontSize="11" fill="var(--muted)">
          已掌握
        </text>
      </svg>
      <div className="legend" style={{ flex: 1 }}>
        {parts.map((p) => (
          <div className="legend-row" key={p.label}>
            <i style={{ background: p.color }} />
            <span>{p.label}</span>
            <b>{p.value}</b>
          </div>
        ))}
      </div>
    </div>
  );
}

function Ratings({ values }: { values: number[] }) {
  const total = values.reduce((a, b) => a + b, 0);
  const meta = [
    { label: "重来", color: "var(--again)" },
    { label: "困难", color: "var(--hard)" },
    { label: "良好", color: "var(--good)" },
    { label: "简单", color: "var(--easy)" },
  ];
  return (
    <div>
      <div className="progress" style={{ height: 14, marginBottom: 20 }}>
        {values.map((v, i) => (
          <span key={i} style={{ width: total ? `${(v / total) * 100}%` : 0, background: meta[i].color }} />
        ))}
      </div>
      <div className="legend">
        {values.map((v, i) => (
          <div className="legend-row" key={i}>
            <i style={{ background: meta[i].color }} />
            <span>{meta[i].label}</span>
            <b>
              {v}
              <span style={{ color: "var(--faint)", fontWeight: 500, marginLeft: 6 }}>
                {total ? Math.round((v / total) * 100) : 0}%
              </span>
            </b>
          </div>
        ))}
      </div>
    </div>
  );
}
