import { createContext, ReactNode, useCallback, useContext, useEffect, useRef, useState } from "react";
import { X } from "lucide-react";

// ------------------------------------------------------------------ Modal

export function Modal(props: {
  title: string;
  icon?: ReactNode;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  wide?: boolean;
  closable?: boolean;
}) {
  const { onClose, closable = true } = props;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && closable) onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, closable]);
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && closable && onClose()}>
      <div className={`modal ${props.wide ? "wide" : ""}`} role="dialog">
        <div className="modal-head">
          {props.icon && <div className="modal-icon">{props.icon}</div>}
          <h2 className="modal-title">{props.title}</h2>
          {closable && (
            <button className="icon-btn" onClick={onClose} aria-label="关闭">
              <X size={18} />
            </button>
          )}
        </div>
        <div className="modal-body">{props.children}</div>
        {props.footer && <div className="modal-foot">{props.footer}</div>}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ Toast

interface ToastItem {
  id: number;
  text: string;
  error?: boolean;
}

const ToastCtx = createContext<(text: string, error?: boolean) => void>(() => {});

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);
  const show = useCallback((text: string, error = false) => {
    const id = Date.now() + Math.random();
    setItems((xs) => [...xs, { id, text, error }]);
    setTimeout(() => setItems((xs) => xs.filter((x) => x.id !== id)), error ? 4500 : 2600);
  }, []);
  return (
    <ToastCtx.Provider value={show}>
      {children}
      <div className="toast-wrap">
        {items.map((t) => (
          <div key={t.id} className={`toast ${t.error ? "error" : ""}`}>
            {t.text}
          </div>
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

export const useToast = () => useContext(ToastCtx);

// ------------------------------------------------------------------ Dropdown menu

export function Menu(props: {
  anchor: HTMLElement;
  onClose: () => void;
  items: ({ label: string; icon?: ReactNode; danger?: boolean; onClick: () => void } | "sep")[];
}) {
  const ref = useRef<HTMLDivElement>(null);
  const rect = props.anchor.getBoundingClientRect();
  const { onClose } = props;
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    setTimeout(() => window.addEventListener("mousedown", onDown), 0);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);
  const left = Math.min(rect.right - 170, window.innerWidth - 186);
  return (
    <div ref={ref} className="menu" style={{ position: "fixed", top: rect.bottom + 6, left: Math.max(8, left) }}>
      {props.items.map((it, i) =>
        it === "sep" ? (
          <hr key={i} />
        ) : (
          <button
            key={i}
            className={it.danger ? "danger" : ""}
            onClick={() => {
              onClose();
              it.onClick();
            }}
          >
            {it.icon}
            {it.label}
          </button>
        ),
      )}
    </div>
  );
}

// ------------------------------------------------------------------ Confirm

export function Confirm(props: {
  title: string;
  message: ReactNode;
  confirmText?: string;
  danger?: boolean;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <Modal
      title={props.title}
      onClose={props.onClose}
      footer={
        <>
          <button className="btn" onClick={props.onClose}>
            取消
          </button>
          <button
            className={`btn ${props.danger ? "btn-danger" : "btn-primary"}`}
            autoFocus
            onClick={() => {
              props.onConfirm();
              props.onClose();
            }}
          >
            {props.confirmText ?? "确定"}
          </button>
        </>
      }
    >
      <div style={{ color: "var(--muted)", lineHeight: 1.7 }}>{props.message}</div>
    </Modal>
  );
}

export function Switch(props: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      className={`switch ${props.on ? "on" : ""}`}
      role="switch"
      aria-checked={props.on}
      onClick={() => props.onChange(!props.on)}
    />
  );
}

export function Range(props: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
}) {
  const pct = ((props.value - props.min) / (props.max - props.min)) * 100;
  return (
    <input
      type="range"
      min={props.min}
      max={props.max}
      step={props.step ?? 1}
      value={props.value}
      style={{ ["--pct" as string]: `${pct}%` }}
      onChange={(e) => props.onChange(Number(e.target.value))}
    />
  );
}
