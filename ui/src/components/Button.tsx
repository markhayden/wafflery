import type { ComponentPropsWithoutRef } from "react";
import styles from "./Button.module.css";

/** Named after shadcn's Button variants (ADR-0001) so a move onto it is a rename, not a rethink:
 * `primary` is the one main action in a panel (Run) and maps to shadcn's `default`; `destructive`
 * removes or clears something the person added (Remove, Dismiss, Clear); `default` here is the
 * plain bordered button, shadcn's `outline`. */
export type ButtonVariant = "default" | "primary" | "destructive";

type ButtonProps = ComponentPropsWithoutRef<"button"> & {
  busy?: boolean;
  variant?: ButtonVariant;
};

export function Button({ busy = false, variant = "default", disabled, children, className, type = "button", ...props }: ButtonProps) {
  return (
    <button
      {...props}
      type={type}
      data-variant={variant === "default" ? undefined : variant}
      className={[styles.button, variant !== "default" && styles[variant], className].filter(Boolean).join(" ")}
      disabled={disabled || busy}
      aria-busy={busy || undefined}
    >
      {children}
    </button>
  );
}
