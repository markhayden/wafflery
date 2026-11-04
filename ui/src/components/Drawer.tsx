import { useEffect, useId, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { Button } from "./Button";
import styles from "./Drawer.module.css";
type Props = {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: ReactNode;
  footer?: ReactNode;
  opener?: HTMLElement | null;
};
// The width a person dragged the drawer to, kept per browser. Storage can be unavailable (a
// private window, blocked site data); the drawer then simply opens at its default width.
const WIDTH_KEY = "wafflery.drawer.width";
const rem = () => Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
const defaultWidth = () => 66 * rem();
const clampWidth = (px: number) => Math.round(Math.min(Math.max(px, 28 * rem()), window.innerWidth - 4 * rem()));
function savedWidth(): number | undefined {
  try { const value = Number(localStorage.getItem(WIDTH_KEY)); return Number.isFinite(value) && value > 0 ? value : undefined; } catch { return undefined; }
}
function saveWidth(px: number | undefined) {
  try { if (px === undefined) localStorage.removeItem(WIDTH_KEY); else localStorage.setItem(WIDTH_KEY, String(px)); } catch { /* not kept */ }
}

export function Drawer({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  opener,
}: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const returnTo = useRef<HTMLElement | null>(null);
  const startedOutside = useRef(false);
  const outside = (node: HTMLDialogElement, x: number, y: number) => {
    const bounds = node.getBoundingClientRect();
    return x < bounds.left || x > bounds.right || y < bounds.top || y > bounds.bottom;
  };
  const id = useId();
  const [width, setWidth] = useState<number | undefined>(savedWidth);
  // A drag starts at a pointer-down and counts only once the pointer has moved, so the clicks of
  // a double-click (which resets the width) never save a width of their own.
  const dragging = useRef<{ x: number; moved: boolean } | undefined>(undefined);
  const resize = (px: number | undefined, keep: boolean) => {
    const next = px === undefined ? undefined : clampWidth(px);
    setWidth(next);
    if (keep) saveWidth(next);
  };
  useEffect(() => {
    const node = dialog.current;
    if (!node) return;
    if (open && !node.open) {
      returnTo.current =
        opener ??
        (document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null);
      node.showModal();
      heading.current?.focus();
    } else if (!open && node.open) {
      node.close();
      if (returnTo.current?.isConnected) returnTo.current.focus();
    }
  }, [open, opener]);
  useEffect(
    () => () => {
      if (returnTo.current?.isConnected) returnTo.current.focus();
    },
    [],
  );
  return (
    <dialog
      ref={dialog}
      className={styles.drawer}
      style={width ? { "--drawer-width": `${width}px` } as CSSProperties : undefined}
      aria-labelledby={`${id}-title`}
      aria-describedby={description ? `${id}-description` : undefined}
      onPointerDown={(event) => {
        startedOutside.current = event.target === event.currentTarget &&
          outside(event.currentTarget, event.clientX, event.clientY);
      }}
      onPointerCancel={() => { startedOutside.current = false; }}
      onClick={(event) => {
        const dismiss = startedOutside.current && event.target === event.currentTarget &&
          outside(event.currentTarget, event.clientX, event.clientY);
        startedOutside.current = false;
        if (dismiss) onClose();
      }}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      {/* Drag the left edge to resize; arrow keys from the keyboard; double-click resets. */}
      <div className={styles.resize} role="separator" aria-orientation="vertical" aria-label="Resize drawer" tabIndex={0}
        aria-valuenow={Math.round(width ?? defaultWidth())} aria-valuemin={Math.round(28 * rem())} aria-valuemax={Math.round(window.innerWidth - 4 * rem())}
        onPointerDown={(event) => { dragging.current = { x: event.clientX, moved: false }; event.currentTarget.setPointerCapture(event.pointerId); event.preventDefault(); }}
        onPointerMove={(event) => {
          const drag = dragging.current;
          if (!drag) return;
          if (!drag.moved && Math.abs(event.clientX - drag.x) < 3) return;
          drag.moved = true;
          resize(window.innerWidth - event.clientX, false);
        }}
        onPointerUp={(event) => {
          const drag = dragging.current;
          dragging.current = undefined;
          if (drag?.moved) resize(window.innerWidth - event.clientX, true);
        }}
        onDoubleClick={() => resize(undefined, true)}
        onKeyDown={(event) => {
          const step = event.shiftKey ? 8 * rem() : 2 * rem();
          const current = width ?? defaultWidth();
          if (event.key === "ArrowLeft") { event.preventDefault(); resize(current + step, true); }
          else if (event.key === "ArrowRight") { event.preventDefault(); resize(current - step, true); }
          else if (event.key === "Home") { event.preventDefault(); resize(undefined, true); }
        }} />
      <header className={styles.header}>
        <div>
          <h2 ref={heading} tabIndex={-1} id={`${id}-title`}>
            {title}
          </h2>
          {description && <p id={`${id}-description`}>{description}</p>}
        </div>
        <Button className={styles.close} onClick={onClose} aria-label="Close" title="Close">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" aria-hidden="true" focusable="false">
            <path d="m6 6 12 12M18 6 6 18" />
          </svg>
        </Button>
      </header>
      <div className={styles.content}>{children}</div>
      {footer && <footer className={styles.footer}>{footer}</footer>}
    </dialog>
  );
}
