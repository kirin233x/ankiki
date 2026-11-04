import { useEffect, useState } from "react";
import { api, CardView } from "../api";
import { useToast } from "./ui";

/** 编辑一条笔记的全部字段（HTML 源码） */
export default function NoteEditor(props: { card: CardView; onSaved?: () => void; onCancel?: () => void }) {
  const toast = useToast();
  const [fields, setFields] = useState<string[]>(props.card.fields);
  const [tags, setTags] = useState(props.card.tags);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setFields(props.card.fields);
    setTags(props.card.tags);
  }, [props.card]);

  const dirty = tags !== props.card.tags || fields.some((f, i) => f !== props.card.fields[i]);

  const save = async () => {
    setSaving(true);
    try {
      await api.updateNote(props.card.noteId, fields, tags);
      toast("已保存");
      props.onSaved?.();
    } catch (e) {
      toast(String(e), true);
    } finally {
      setSaving(false);
    }
  };

  const names = props.card.notetype.fields;
  const count = Math.max(names.length, fields.length);

  return (
    <div>
      {Array.from({ length: count }, (_, i) => (
        <div className="field" key={i} style={{ marginBottom: 12 }}>
          <div className="field-label">{names[i] ?? `字段 ${i + 1}`}</div>
          <textarea
            className="textarea"
            style={{ minHeight: i === 0 ? 44 : 64, fontFamily: "ui-monospace, Menlo, monospace", fontSize: 12.5 }}
            value={fields[i] ?? ""}
            onChange={(e) => setFields((fs) => {
              const next = [...fs];
              while (next.length <= i) next.push("");
              next[i] = e.target.value;
              return next;
            })}
          />
        </div>
      ))}
      <div className="field" style={{ marginBottom: 14 }}>
        <div className="field-label">标签</div>
        <input className="input" value={tags} onChange={(e) => setTags(e.target.value)} placeholder="空格分隔" />
      </div>
      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
        {props.onCancel && (
          <button className="btn" onClick={props.onCancel}>
            取消
          </button>
        )}
        <button className="btn btn-primary" disabled={!dirty || saving} onClick={save}>
          保存修改
        </button>
      </div>
    </div>
  );
}
