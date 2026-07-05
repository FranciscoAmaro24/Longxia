import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "../../lib/cn";
import styles from "./Panel.module.css";

export interface PanelProps extends HTMLAttributes<HTMLDivElement> {
  /** Optional mono uppercase eyebrow shown in the header. */
  label?: ReactNode;
  /** Optional actions (e.g. a Button) rendered at the header's end. */
  actions?: ReactNode;
  /** Set false to render children flush without body padding. */
  padded?: boolean;
}

/**
 * A bordered surface. Deliberately flat (hairline border, no drop shadow) to
 * avoid the "card soup" look. Header only renders when `label` or `actions`
 * are provided.
 */
export const Panel = forwardRef<HTMLDivElement, PanelProps>(
  ({ label, actions, padded = true, className, children, ...rest }, ref) => {
    const hasHeader = label != null || actions != null;
    return (
      <div ref={ref} className={cn(styles.panel, className)} {...rest}>
        {hasHeader && (
          <div className={styles.header}>
            {label != null && <span className={styles.label}>{label}</span>}
            {actions != null && <div className={styles.actions}>{actions}</div>}
          </div>
        )}
        <div className={padded ? styles.body : undefined}>{children}</div>
      </div>
    );
  },
);

Panel.displayName = "Panel";
