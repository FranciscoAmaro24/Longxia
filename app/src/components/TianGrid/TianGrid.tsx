import {
  forwardRef,
  type CSSProperties,
  type HTMLAttributes,
  type ReactNode,
} from "react";
import { cn } from "../../lib/cn";
import styles from "./TianGrid.module.css";

export interface TianGridProps extends HTMLAttributes<HTMLDivElement> {
  /** Cell size in pixels. Drives both the box and the glyph. */
  size?: number;
  /** A character to show centred in the cell. */
  char?: string;
  /** `ghost` = faint model glyph to trace; `ink` = solid. */
  tone?: "ghost" | "ink";
  /** Custom content (e.g. a canvas or animation) instead of a static char. */
  children?: ReactNode;
}

/**
 * The 田字格 practice cell. Purely presentational: pass `char` for a static
 * glyph, or `children` to mount a drawing surface later (Step: Writing).
 */
export const TianGrid = forwardRef<HTMLDivElement, TianGridProps>(
  ({ size = 96, char, tone = "ink", children, className, style, ...rest }, ref) => (
    <div
      ref={ref}
      className={cn(styles.tian, className)}
      style={{ ["--tian-size"]: `${size}px`, ...style } as CSSProperties}
      {...rest}
    >
      {children ??
        (char != null && (
          <span className={cn(styles.char, styles[tone])} lang="zh">
            {char}
          </span>
        ))}
    </div>
  ),
);

TianGrid.displayName = "TianGrid";
