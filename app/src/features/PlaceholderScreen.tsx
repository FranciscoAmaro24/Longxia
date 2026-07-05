import { Panel, Tag } from "../components";
import styles from "./PlaceholderScreen.module.css";

export interface PlaceholderScreenProps {
  zh: string;
  en: string;
  /** One-line note on what this section will hold. */
  note?: string;
}

/**
 * Stand-in for sections not yet built. Keeps the shell fully navigable during
 * development; each is replaced by its real feature screen in a later step.
 */
export function PlaceholderScreen({ zh, en, note }: PlaceholderScreenProps) {
  return (
    <div className={styles.screen}>
      <header className={styles.header}>
        <span className={styles.zh} lang="zh">
          {zh}
        </span>
        <span className={styles.en}>{en}</span>
      </header>
      <Panel label={en} actions={<Tag>Planned</Tag>}>
        <p className={styles.body}>{note ?? "This section is coming in a later step."}</p>
      </Panel>
    </div>
  );
}
