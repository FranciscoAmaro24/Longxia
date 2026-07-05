/**
 * Tiny classnames joiner. Filters out falsy values so callers can write
 * `cn(styles.base, active && styles.active, className)` without noise.
 * Kept dependency-free on purpose.
 */
export function cn(
  ...parts: Array<string | false | null | undefined>
): string {
  return parts.filter(Boolean).join(" ");
}
