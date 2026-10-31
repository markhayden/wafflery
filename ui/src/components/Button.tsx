import type { ComponentPropsWithoutRef } from "react";
import styles from "./Button.module.css";

type ButtonProps = ComponentPropsWithoutRef<"button"> & {
  busy?: boolean;
};

export function Button({ busy = false, disabled, children, className, type = "button", ...props }: ButtonProps) {
  return (
    <button
      {...props}
      type={type}
      className={[styles.button, className].filter(Boolean).join(" ")}
      disabled={disabled || busy}
      aria-busy={busy || undefined}
    >
      {children}
    </button>
  );
}
