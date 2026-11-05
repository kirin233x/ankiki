import { useEffect, useState } from "react";
import { CheckCircle2, FolderOpen, RefreshCw, TriangleAlert, UserRound } from "lucide-react";
import { AnkiProfile, api, ImportProgress, onImportProgress, pickAnkiCollection, SyncResult } from "../api";
import { Modal } from "./ui";
import { formatDateTime } from "../utils";

type Phase = "pick" | "running" | "done" | "error";

export default function SyncDialog(props: { onClose: () => void; onDone: () => void }) {
  const [phase, setPhase] = useState<Phase>("pick");
  const [profiles, setProfiles] = useState<AnkiProfile[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [stage, setStage] = useState("正在读取 Anki 数据…");
  const [result, setResult] = useState<SyncResult | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    api
      .findAnkiProfiles()
      .then((ps) => {
        setProfiles(ps);
        if (ps.length) setSelected(ps[0].path);
      })
      .catch(() => setProfiles([]));
    let off: (() => void) | undefined;
    onImportProgress((p: ImportProgress) => setStage(p.stage)).then((f) => (off = f));
    return () => off?.();
  }, []);

  const run = async (path: string) => {
    setPhase("running");
    try {
      setResult(await api.syncFromAnki(path));
      setPhase("done");
      props.onDone();
    } catch (e) {
      setError(String(e));
      setPhase("error");
    }
  };

  const browse = async () => {
    const p = await pickAnkiCollection();
    if (p) run(p);
  };

  if (phase === "running") {
    return (
      <Modal title="正在同步" icon={<RefreshCw size={20} />} onClose={() => {}} closable={false}>
        <div className="import-progress">
          <div className="spinner" />
          <div style={{ fontWeight: 600, marginBottom: 8 }}>{stage}</div>
          <div style={{ color: "var(--muted)", fontSize: 12 }}>大的 Anki 数据库可能需要几十秒</div>
        </div>
      </Modal>
    );
  }

  if (phase === "done" && result) {
    return (
      <Modal
        title="同步完成"
        icon={<CheckCircle2 size={20} />}
        onClose={props.onClose}
        footer={
          <button className="btn btn-primary" onClick={props.onClose}>
            好的
          </button>
        }
      >
        <div className="result-grid">
          <div className="result-item">
            <b>{result.updatedCards}</b>
            <span>张卡片更新了学习进度</span>
          </div>
          <div className="result-item">
            <b>{result.revlogAdded}</b>
            <span>条复习记录</span>
          </div>
          <div className="result-item">
            <b>{result.matchedNotes}</b>
            <span>个单词与 Anki 匹配</span>
          </div>
          <div className="result-item">
            <b>{result.skippedNewer}</b>
            <span>张卡片 Ankiki 中更新，保持不变</span>
          </div>
        </div>
        {result.missingNotes > 0 && (
          <p style={{ fontSize: 12.5, color: "var(--muted)", marginTop: 14, lineHeight: 1.7 }}>
            Anki 中还有 {result.missingNotes} 个单词不在 Ankiki 里（属于其他尚未导入的词书）。
            如需同步，请先导入对应词书再同步一次。
          </p>
        )}
      </Modal>
    );
  }

  if (phase === "error") {
    return (
      <Modal
        title="同步失败"
        icon={<TriangleAlert size={20} />}
        onClose={props.onClose}
        footer={
          <>
            <button className="btn" onClick={() => setPhase("pick")}>
              返回
            </button>
            <button className="btn btn-primary" onClick={props.onClose}>
              关闭
            </button>
          </>
        }
      >
        <p style={{ margin: 0, lineHeight: 1.7 }}>{error}</p>
      </Modal>
    );
  }

  return (
    <Modal
      title="从 Anki 同步学习进度"
      icon={<RefreshCw size={20} />}
      onClose={props.onClose}
      footer={
        <>
          <button className="btn btn-ghost" style={{ marginRight: "auto" }} onClick={browse}>
            <FolderOpen size={15} />
            手动选择文件…
          </button>
          <button className="btn" onClick={props.onClose}>
            取消
          </button>
          <button className="btn btn-primary" disabled={!selected} onClick={() => selected && run(selected)}>
            <RefreshCw size={15} />
            开始同步
          </button>
        </>
      }
    >
      <p style={{ margin: "0 0 14px", color: "var(--muted)", lineHeight: 1.7, fontSize: 13 }}>
        直接读取本机 Anki 的数据（只读，不会改动 Anki），把你在 Anki 里背过的单词进度和复习记录同步到 Ankiki 中
        <b style={{ color: "var(--text)" }}>已导入的同一本词书</b>。两边都学过的卡片，以最近一次复习的一方为准。
      </p>
      {profiles === null ? (
        <div className="spinner" style={{ width: 28, height: 28 }} />
      ) : profiles.length === 0 ? (
        <div className="check">
          <TriangleAlert size={20} color="var(--hard)" />
          <div className="check-text">
            <b>没有找到 Anki 数据</b>
            <span>可以点「手动选择文件」，找到 Anki 用户文件夹里的 collection.anki2</span>
          </div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {profiles.map((p) => (
            <button
              key={p.path}
              className="check"
              style={{
                textAlign: "left",
                cursor: "pointer",
                borderColor: selected === p.path ? "var(--accent)" : undefined,
                background: selected === p.path ? "var(--accent-soft)" : undefined,
              }}
              onClick={() => setSelected(p.path)}
            >
              <UserRound size={20} color="var(--accent)" />
              <div className="check-text">
                <b>Anki 用户「{p.name}」</b>
                <span>最后修改：{formatDateTime(p.modified * 1000)}</span>
              </div>
            </button>
          ))}
        </div>
      )}
      <div className="field-help" style={{ marginTop: 14 }}>
        提示：同步前请先在 Anki 中完成学习并<b>退出 Anki</b>，确保最新进度已经保存。
      </div>
    </Modal>
  );
}
