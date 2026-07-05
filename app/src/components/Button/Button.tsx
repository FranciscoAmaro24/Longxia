import { forwardRef, type ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";
import styles from "./Button.module.css";

export type ButtonVariant =
  | "primary"
  | "secondary"
  | "ghost"
  | "quiet"
  | "accent";

export type ButtonSize = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
}

/**
 * The one button primitive. Square, no gradients, no pill. Variants map to a
 * clear intent hierarchy (primary > secondary > ghost > quiet), plus `accent`
 * reserved for red-pen / AI actions. Forwards refs and spreads native button
 * props so it stays a drop-in for `<button>`.
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "secondary", size = "md", type, className, ...rest }, ref) => (
    <button
      ref={ref}
      // default to type="button" so it never accidentally submits a form
      type={type ?? "button"}
      className={cn(styles.btn, styles[variant], styles[size], className)}
      {...rest}
    />
  ),
);

Button.displayName = "Button";
