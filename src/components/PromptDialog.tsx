import { useState } from "react";
import { PencilLine } from "lucide-react";
import { Modal, useToast } from "./ui";

export default function PromptDialog(props: {
  title: string;
  label: string;
  placeholder?: string;
  initial?: string;
  confirmText?: string;
  onSubmit: (value: string) => Promise<void>;
  onClose: () => void;
}) {
  const [value, setValue] = useState(props.initial ?? "");
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const submit = async () => {
    if (!value.trim() || busy) return;
    setBusy(true);
    try {
      await props.onSubmit(value.trim());
      props.onClose();
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      title={props.title}
      icon={<PencilLine size={20} />}
      onClose={props.onClose}
      footer={
        <>
          <button className="btn" onClick={props.onClose}>
            取消
          </button>
          <button className="btn btn-primary" onClick={submit} disabled={!value.trim() || busy}>
            {props.confirmText ?? "确定"}
          </button>
        </>
      }
    >
      <div className="field">
        <div className="field-label">{props.label}</div>
        <input
          className="input"
          autoFocus
          value={value}
          placeholder={props.placeholder}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && submit()}
        />
      </div>
    </Modal>
  );
}
