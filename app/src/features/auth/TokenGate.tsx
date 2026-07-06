import { useState, type FormEvent } from "react";
import { Button, Panel } from "../../components";
import { setApiToken } from "../../lib/api";
import styles from "./TokenGate.module.css";

/**
 * Access gate for the web build. When the app is served over the network,
 * `longxia-server` requires a shared token on every data request. This asks for
 * it once and stores it (via `setApiToken`), so the token never ships in the
 * bundle. It is only rendered in the browser when no token is present; the Tauri
 * app talks to the local core directly and never sees this.
 */
export function TokenGate({ onAuthed }: { onAuthed: () => void }) {
  const [value, setValue] = useState("");

  function submit(e: FormEvent) {
    e.preventDefault();
    const token = value.trim();
    if (!token) return;
    setApiToken(token);
    onAuthed();
  }

  return (
    <div className={styles.wrap}>
      <Panel label="访问 · access" className={styles.panel}>
        <form className={styles.form} onSubmit={submit}>
          <p className={styles.lead}>
            This copy of <span className={styles.mark}>龙虾</span> is served over the web. Enter the
            access token you were given to continue.
          </p>
          <label className={styles.field}>
            <span className={styles.fieldLabel}>Access token</span>
            <input
              className={styles.input}
              type="password"
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder="paste token"
              autoFocus
              spellCheck={false}
              autoComplete="off"
            />
          </label>
          <div className={styles.actions}>
            <Button type="submit" variant="primary" disabled={!value.trim()}>
              Continue
            </Button>
          </div>
        </form>
      </Panel>
    </div>
  );
}
