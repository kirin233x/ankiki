import { useEffect, useState } from "react";
import { SlidersHorizontal } from "lucide-react";
import { api, DeckConfig, DeckInfo } from "../api";
import { Modal, Range, useToast } from "./ui";
import { useApp } from "../App";

function parseSteps(s: string): number[] {
  return s
    .split(/[\s,，]+/)
    .map((x) => {
      const m = x.trim().match(/^(\d+(?:\.\d+)?)\s*([mhd分时天]?)$/i);
      if (!m) return NaN;
      const v = parseFloat(m[1]);
      const unit = m[2].toLowerCase();
      if (unit === "h" || unit === "时") return v * 60;
      if (unit === "d" || unit === "天") return v * 1440;
      return v;
    })
    .filter((v) => v > 0);
}

function stepsText(steps: number[]): string {
  return steps
    .map((m) => (m >= 1440 && m % 1440 === 0 ? `${m / 1440}d` : m >= 60 && m % 60 === 0 ? `${m / 60}h` : `${m}m`))
    .join(" ");
}

export default function DeckOptionsDialog(props: { deck: DeckInfo; onClose: () => void }) {
  const { refreshDecks } = useApp();
  const toast = useToast();
  const [cfg, setCfg] = useState<DeckConfig>(props.deck.config);
  const [learn, setLearn] = useState(stepsText(props.deck.config.learningSteps));
  const [relearn, setRelearn] = useState(stepsText(props.deck.config.relearningSteps));
  const [desc, setDesc] = useState(props.deck.description);
  const [fieldNames, setFieldNames] = useState<string[]>([]);
  useEffect(() => {
    api.deckFields(props.deck.id).then(setFieldNames).catch(() => {});
  }, [props.deck.id]);
  const set = <K extends keyof DeckConfig>(k: K, v: DeckConfig[K]) => setCfg((c) => ({ ...c, [k]: v }));

  const save = async () => {
    try {
      await api.updateDeckConfig(props.deck.id, {
        ...cfg,
        learningSteps: parseSteps(learn),
        relearningSteps: parseSteps(relearn),
      });
      if (desc !== props.deck.description) await api.setDeckDescription(props.deck.id, desc);
      await refreshDecks();
      toast("设置已保存");
      props.onClose();
    } catch (e) {
      toast(String(e), true);
    }
  };

  const minutesPerDay = Math.round(cfg.newPerDay * 1.2 + cfg.newPerDay * 4 * 0.15);

  return (
    <Modal
      wide
      title={`「${props.deck.name}」学习设置`}
      icon={<SlidersHorizontal size={20} />}
      onClose={props.onClose}
      footer={
        <>
          <button className="btn" onClick={props.onClose}>
            取消
          </button>
          <button className="btn btn-primary" onClick={save}>
            保存设置
          </button>
        </>
      }
    >
      <div className="field">
        <div className="field-label">
          每天学习新词 <em>{cfg.newPerDay} 个</em>
        </div>
        <Range value={cfg.newPerDay} min={0} max={200} step={5} onChange={(v) => set("newPerDay", v)} />
        <div className="field-help">
          稳定之后，每天的复习量大约是新词数的 5~10 倍，预计每天需要约 {minutesPerDay} 分钟。
        </div>
      </div>
      <div className="field">
        <div className="field-label">
          每天最多复习 <em>{cfg.reviewsPerDay} 张</em>
        </div>
        <Range value={cfg.reviewsPerDay} min={20} max={2000} step={10} onChange={(v) => set("reviewsPerDay", v)} />
      </div>
      <div className="field">
        <div className="field-label">
          目标记忆保持率 <em>{Math.round(cfg.desiredRetention * 100)}%</em>
        </div>
        <Range
          value={Math.round(cfg.desiredRetention * 100)}
          min={75}
          max={97}
          onChange={(v) => set("desiredRetention", v / 100)}
        />
        <div className="field-help">
          FSRS 算法会让你在复习时大约能记住这个比例的单词。越高复习越频繁；推荐 85%~92%。
        </div>
      </div>
      <div className="row">
        <div className="field">
          <div className="field-label">学习步长</div>
          <input className="input" value={learn} onChange={(e) => setLearn(e.target.value)} placeholder="1m 10m" />
          <div className="field-help">新词在当天重复出现的间隔，如 1m 10m</div>
        </div>
        <div className="field">
          <div className="field-label">重学步长</div>
          <input className="input" value={relearn} onChange={(e) => setRelearn(e.target.value)} placeholder="10m" />
          <div className="field-help">忘记的单词再次出现的间隔</div>
        </div>
      </div>
      <div className="row">
        <div className="field">
          <div className="field-label">新词顺序</div>
          <select className="select" value={cfg.newOrder} onChange={(e) => set("newOrder", Number(e.target.value))}>
            <option value={0}>按词书顺序</option>
            <option value={1}>随机打乱</option>
          </select>
        </div>
        <div className="field">
          <div className="field-label">最长间隔（天）</div>
          <input
            className="input"
            type="number"
            min={1}
            max={36500}
            value={cfg.maxInterval}
            onChange={(e) => set("maxInterval", Number(e.target.value) || 36500)}
          />
        </div>
      </div>
      <div className="field">
        <div className="field-label">词表看板中的释义字段</div>
        <select
          className="select"
          value={cfg.meaningField ?? ""}
          onChange={(e) => set("meaningField", e.target.value)}
        >
          <option value="">自动识别（第一个含中文的字段）</option>
          {fieldNames.map((f) => (
            <option key={f} value={f}>
              {f}
            </option>
          ))}
        </select>
        <div className="field-help">如果词表看板里显示的不是中文释义，可以在这里指定正确的字段。</div>
      </div>
      <div className="field" style={{ marginBottom: 6 }}>
        <div className="field-label">词书简介</div>
        <input className="input" value={desc} placeholder="可选" onChange={(e) => setDesc(e.target.value)} />
      </div>
    </Modal>
  );
}
