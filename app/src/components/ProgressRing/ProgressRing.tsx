import { type CSSProperties } from "react";
import { cn } from "../../lib/cn";
import styles from "./ProgressRing.module.css";

export interface ProgressRingProps {
  /** Progress 0-100. Clamped defensively. */
  value: number;
  /** Diameter in px. */
  size?: number;
  /** Ring thickness in px. */
  thickness?: number;
  /** Chinese caption under the ring (e.g. the category). */
  zh?: string;
  /** Small mono count under the caption (e.g. "674 / 900"). */
  count?: string;
  className?: string;
}

const clamp = (n: number) => Math.max(0, Math.min(100, n));

/**
 * Radial progress gauge. Presentational and reusable: it renders whatever
 * `value` it is given and labels it with a number, so it never conveys state
 * by color alone.
 */
export function ProgressRing({
  value,
  size = 64,
  thickness = 7,
  zh,
  count,
  className,
}: ProgressRingProps) {
  const pct = Math.round(clamp(value));
  const ringStyle = {
    ["--ring-size"]: `${size}px`,
    ["--ring-thickness"]: `${thickness}px`,
    ["--value"]: pct,
  } as CSSProperties;

  const aria = [zh, `${pct}%`].filter(Boolean).join(" ");

  return (
    <div className={cn(styles.wrap, className)}>
      <div
        className={styles.ring}
        style={ringStyle}
        role="img"
        aria-label={aria}
      >
        <div className={styles.hole}>
          <span className={styles.pct}>{pct}</span>
        </div>
      </div>
      {(zh || count) && (
        <div className={styles.caption}>
          {zh && (
            <span className={styles.zh} lang="zh">
              {zh}
            </span>
          )}
          {count && <span className={styles.count}>{count}</span>}
        </div>
      )}
    </div>
  );
}
