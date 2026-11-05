import { useEffect, useState } from "react";
import { CheckCircle2, Download, FileArchive, TriangleAlert } from "lucide-react";
import { api, ImportProgress, ImportResult, onImportProgress } from "../api";
import { Modal, Switch } from "./ui";
import { fileName } from "../utils";
import { useApp } from "../App";

type Phase = "confirm" | "running" | "done" | "error";

export default function ImportDialog(props: { path: string; onClose: () => void; onDone: () => void }) {
  const { go } = useApp();
  const [phase, setPhase] = useState<Phase>("confirm");
  const [includeProgress, setIncludeProgress] = useState(true);
  const [deckName, setDeckName] = useState("");
  const [progress, setProgress] = useState<ImportProgress>({ stage: "准备中…", current: 0, total: 0 });
  const [result, setResult] = useState<ImportResult | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let off: (() => void) | undefined;
    onImportProgress(setProgress).then((f) => (off = f));
    return () => off?.();
  }, []);

  const run = async () => {
    setPhase("running");
    try {
      const r = await api.importPackage(props.path, { includeProgress, deckName: deckName.trim() || null });
      setResult(r);
      setPhase("done");
      props.onDone();
    } catch (e) {
      setError(String(e));
      setPhase("error");
    }
  };

  const pct = progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0;

  if (phase === "running") {
    return (
      <Modal title="正在导入" icon={<Download size={20} />} onClose={() => {}} closable={false}>
        <div className="import-progress">
          <div className="spinner" />
          <div style={{ fontWeight: 600, marginBottom: 12 }}>{progress.stage}</div>
          <div className="progress" style={{ marginBottom: 8 }}>
            <span style={{ width: `${pct}%`, background: "var(--grad)" }} />
          </div>
          <div style={{ color: "var(--muted)", fontSize: 12 }}>
            {progress.total > 0 ? `${progress.current} / ${progress.total}` : "大词书可能需要几十秒，请稍候"}
          </div>
        </div>
      </Modal>
    );
  }

  if (phase === "done" && result) {
    return (
      <Modal
        title={result.notesAdded > 0 ? "导入成功" : "学习进度已更新"}
        icon={<CheckCircle2 size={20} />}
        onClose={props.onClose}
        footer={
          <>
            <button className="btn" onClick={props.onClose}>
              完成
            </button>
            <button
              className="btn btn-primary"
              onClick={() => {
                props.onClose();
                go({ name: "study", deckId: result.deckId });
              }}
            >
              立即开始学习
            </button>
          </>
        }
      >
        <p style={{ margin: "0 0 14px", color: "var(--muted)" }}>
          {result.notesAdded > 0 ? (
            <>
              「<b style={{ color: "var(--text)" }}>{result.deckName}</b>」已加入你的词书。
            </>
          ) : (
            <>
              这本词书之前已经导入过，已用其中较新的学习进度更新了「
              <b style={{ color: "var(--text)" }}>{result.deckName}</b>」中的 {result.progressUpdated} 张卡片。
            </>
          )}
        </p>
        <div className="result-grid">
          <div className="result-item">
            <b>{result.notesAdded}</b>
            <span>个单词</span>
          </div>
          <div className="result-item">
            <b>{result.cardsAdded}</b>
            <span>张卡片</span>
          </div>
          <div className="result-item">
            <b>{result.mediaFiles}</b>
            <span>个音频 / 图片</span>
          </div>
          <div className="result-item">
            <b>{result.withProgress}</b>
            <span>张卡片保留了学习进度</span>
          </div>
        </div>
        {result.duplicates > 0 && (
          <p style={{ fontSize: 12.5, color: "var(--muted)", marginTop: 14 }}>
            有 {result.duplicates} 个单词之前已经导入过，没有重复添加
            {result.progressUpdated > 0 ? `，其中 ${result.progressUpdated} 张卡片的学习进度已更新` : ""}。
          </p>
        )}
      </Modal>
    );
  }

  if (phase === "error") {
    return (
      <Modal
        title="导入失败"
        icon={<TriangleAlert size={20} />}
        onClose={props.onClose}
        footer={
          <button className="btn" onClick={props.onClose}>
            关闭
          </button>
        }
      >
        <p style={{ margin: 0, lineHeight: 1.7 }}>{error}</p>
        <p style={{ fontSize: 12.5, color: "var(--muted)", lineHeight: 1.7 }}>
          提示：请确认文件是从 Anki「导出」得到的 .apkg 词书。如果仍然失败，可以在 Anki 导出时勾选「支持旧版本 Anki」后重试。
        </p>
      </Modal>
    );
  }

  return (
    <Modal
      title="导入 Anki 词书"
      icon={<FileArchive size={20} />}
      onClose={props.onClose}
      footer={
        <>
          <button className="btn" onClick={props.onClose}>
            取消
          </button>
          <button className="btn btn-primary" onClick={run} autoFocus>
            <Download size={16} />
            开始导入
          </button>
        </>
      }
    >
      <div className="check" style={{ marginBottom: 18 }}>
        <FileArchive size={22} color="var(--accent)" />
        <div className="check-text">
          <b style={{ wordBreak: "break-all" }}>{fileName(props.path)}</b>
          <span>支持 Anki 2.0 / 2.1 / 新版（.apkg、.colpkg）</span>
        </div>
      </div>
      <div className="field">
        <div className="field-label">词书名称</div>
        <input
          className="input"
          placeholder="留空则自动使用词书原名"
          value={deckName}
          onChange={(e) => setDeckName(e.target.value)}
        />
      </div>
      <div className="check">
        <div className="check-text">
          <b>保留学习进度</b>
          <span>导入词书里的 Anki 复习进度与记录；已导入过的单词会更新为较新的进度</span>
        </div>
        <Switch on={includeProgress} onChange={setIncludeProgress} />
      </div>
    </Modal>
  );
}
