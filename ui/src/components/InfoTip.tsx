import { useEffect, useId, useRef } from "react";
import styles from "./InfoTip.module.css";

/** An ⓘ button that opens a short explanation under it (a popover: outside click or Escape closes). */
export function InfoTip({ label, children }: { label: string; children: React.ReactNode }) {
  const id = useId();
  const button = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const tip = panel.current;
    if (!tip) return;
    const place = () => {
      const anchor = button.current?.getBoundingClientRect();
      if (!anchor) return;
      const rem = Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
      const edge = rem, width = Math.min(22 * rem, window.innerWidth - 2 * edge);
      tip.style.width = `${width}px`;
      tip.style.left = `${Math.min(Math.max(anchor.left - rem, edge), window.innerWidth - edge - width)}px`;
      const below = window.innerHeight - anchor.bottom;
      const up = below < 12 * rem && anchor.top > below;
      tip.style.top = up ? "auto" : `${anchor.bottom + 6}px`;
      tip.style.bottom = up ? `${window.innerHeight - anchor.top + 6}px` : "auto";
    };
    // beforetoggle fires before the popover paints, so it never shows a frame unplaced.
    const opening = (event: Event) => { if ((event as ToggleEvent).newState === "open") place(); };
    const follow = () => { if (tip.matches(":popover-open")) place(); };
    tip.addEventListener("beforetoggle", opening);
    window.addEventListener("resize", follow);
    window.addEventListener("scroll", follow, true);
    return () => {
      tip.removeEventListener("beforetoggle", opening);
      window.removeEventListener("resize", follow);
      window.removeEventListener("scroll", follow, true);
    };
  }, []);
  return <>
    <button ref={button} type="button" className={styles.button} popoverTarget={`${id}-tip`} aria-label={`About ${label}`}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
        <circle cx="12" cy="12" r="9.25" /><path d="M12 11v5.5" /><circle cx="12" cy="7.75" r=".6" fill="currentColor" />
      </svg>
    </button>
    <div ref={panel} id={`${id}-tip`} popover="auto" role="note" aria-label={`About ${label}`} className={styles.tip}>
      <strong>{label}</strong>
      <p>{children}</p>
    </div>
  </>;
}
