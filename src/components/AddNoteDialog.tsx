import { useState } from "react";
import { BookPlus } from "lucide-react";
import { api } from "../api";
import { Modal, useToast } from "./ui";
import { useApp } from "../App";

export default function AddNoteDialog(props: { initialDeck: number | null; onClose: () => void }) {
  const { decks, refreshDecks } = useApp();
  const toast = useToast();
  const [deckId, setDeckId] = useState<number>(props.initialDeck ?? decks[0]?.id ?? 0);
  const [front, setFront] = useState("");
  const [back, setBack] = useState("");
  const [tags, setTags] = useState("");
  const [count, setCount] = useState(0);

  const toHtml = (s: string) =>
    s
      .trim()
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/\n/g, "<br>");

  const submit = async () => {
    if (!front.trim() || !deckId) return;
    try {
      await api.addNote(deckId, toHtml(front), toHtml(back), tags);
      setCount((c) => c + 1);
      setFront("");
      setBack("");
      toast(`已添加「${front.trim()}」`);
      refreshDecks();
      document.getElementById("add-front")?.focus();
    } catch (e) {
      toast(String(e), true);
    }
  };

  return (
    <Modal
      title="添加单词"
      icon={<BookPlus size={20} />}
      onClose={props.onClose}
      footer={
        <>
          <span style={{ marginRight: "auto", color: "var(--muted)", fontSize: 12.5, alignSelf: "center" }}>
            {count > 0 ? `本次已添加 ${count} 个` : <>按 <kbd>⌘</kbd> <kbd>↵</kbd> 快速添加</>}
          </span>
          <button className="btn" onClick={props.onClose}>
            完成
          </button>
          <button className="btn btn-primary" onClick={submit} disabled={!front.trim()}>
            添加
          </button>
        </>
      }
    >
      <div
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            submit();
          }
        }}
      >
        <div className="field">
          <div className="field-label">添加到</div>
          <select className="select" value={deckId} onChange={(e) => setDeckId(Number(e.target.value))}>
            {decks.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>
        </div>
        <div className="field">
          <div className="field-label">正面（单词）</div>
          <input
            id="add-front"
            className="input"
            autoFocus
            placeholder="例如：serendipity"
            value={front}
            onChange={(e) => setFront(e.target.value)}
            style={{ fontSize: 16, fontWeight: 600 }}
          />
        </div>
        <div className="field">
          <div className="field-label">背面（释义、例句）</div>
          <textarea
            className="textarea"
            placeholder={"n. 意外发现珍奇事物的本领\nFinding this café was pure serendipity."}
            value={back}
            onChange={(e) => setBack(e.target.value)}
          />
        </div>
        <div className="field" style={{ marginBottom: 4 }}>
          <div className="field-label">标签（可选，空格分隔）</div>
          <input className="input" placeholder="例如：GRE 高频" value={tags} onChange={(e) => setTags(e.target.value)} />
        </div>
      </div>
    </Modal>
  );
}
