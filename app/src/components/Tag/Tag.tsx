import { forwardRef, type HTMLAttributes } from "react";
import { cn } from "../../lib/cn";
import styles from "./Tag.module.css";

export type TagVariant = "default" | "correction" | "jade" | "ink";

export interface TagProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: TagVariant;
}

/**
 * Small mono uppercase label. Color carries meaning: `correction` for
 * red-pen / AI, `jade` for progress / correct. Keep semantics consistent so
 * color stays legible across the app.
 */
export const Tag = forwardRef<HTMLSpanElement, TagProps>(
  ({ variant = "default", className, ...rest }, ref) => (
    <span
      ref={ref}
      className={cn(
        styles.tag,
        variant !== "default" && styles[variant],
        className,
      )}
      {...rest}
    />
  ),
);

Tag.displayName = "Tag";
