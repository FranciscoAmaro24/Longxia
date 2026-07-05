import { type ReactNode } from "react";
import { cn } from "../../lib/cn";
import { NAV_ITEMS, type SectionId } from "../nav";
import styles from "./AppShell.module.css";

export interface AppShellProps {
  active: SectionId;
  onSelect: (id: SectionId) => void;
  children: ReactNode;
}

/**
 * App layout: a sticky left rail (the notebook margin) with section nav, and a
 * scrolling content area. Pure layout - it owns no screen state; the caller
 * decides what to render for `active`.
 */
export function AppShell({ active, onSelect, children }: AppShellProps) {
  return (
    <div className={styles.shell}>
      <nav className={styles.rail} aria-label="Sections">
        <div className={styles.brand}>
          <span className={styles.brandZh} lang="zh">
            龙虾
          </span>
          <span className={styles.brandPy}>Lóngxiā</span>
        </div>

        <div className={styles.nav}>
          {NAV_ITEMS.map((item) => {
            const isActive = item.id === active;
            return (
              <button
                key={item.id}
                type="button"
                className={cn(
                  styles.navItem,
                  isActive && styles.navItemActive,
                )}
                aria-current={isActive ? "page" : undefined}
                onClick={() => onSelect(item.id)}
              >
                <span className={styles.navZh} lang="zh">
                  {item.zh}
                </span>
                <span className={styles.navEn}>{item.en}</span>
              </button>
            );
          })}
        </div>

        <div className={styles.railFoot}>HSK 3.0 · zh-Hans</div>
      </nav>

      <div className={styles.content}>{children}</div>
    </div>
  );
}
