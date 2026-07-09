import { useState, type FormEvent } from "react";
import { Button, Panel } from "../../components";
import { login, signup } from "../../lib/api";
import styles from "./AuthGate.module.css";

type Mode = "login" | "signup";

/**
 * Account gate for the web build. `longxia-server` requires a session on every
 * data request; this signs the user in (or creates an account) and stores the
 * returned session token via the transport, so it never ships in the bundle.
 * Only rendered in the browser - the Tauri app talks to the local core directly.
 */
export function AuthGate({ onAuthed }: { onAuthed: () => void }) {
  const [mode, setMode] = useState<Mode>("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [invite, setInvite] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      if (mode === "login") {
        await login(email.trim(), password);
      } else {
        await signup(email.trim(), password, invite.trim() || undefined);
      }
      onAuthed();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }

  const isLogin = mode === "login";

  return (
    <div className={styles.wrap}>
      <Panel label={isLogin ? "登录 · sign in" : "注册 · sign up"} className={styles.panel}>
        <form className={styles.form} onSubmit={submit}>
          <p className={styles.lead}>
            <span className={styles.mark}>龙虾</span> on the web.{" "}
            {isLogin ? "Sign in to continue." : "Create an account to continue."}
          </p>

          <label className={styles.field}>
            <span className={styles.fieldLabel}>Email</span>
            <input
              className={styles.input}
              type="email"
              autoComplete="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              autoFocus
              required
            />
          </label>

          <label className={styles.field}>
            <span className={styles.fieldLabel}>Password</span>
            <input
              className={styles.input}
              type="password"
              autoComplete={isLogin ? "current-password" : "new-password"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </label>

          {!isLogin && (
            <label className={styles.field}>
              <span className={styles.fieldLabel}>Invite code (if required)</span>
              <input
                className={styles.input}
                type="text"
                value={invite}
                onChange={(e) => setInvite(e.target.value)}
                spellCheck={false}
                autoComplete="off"
              />
            </label>
          )}

          {error && <p className={styles.error}>{error}</p>}

          <div className={styles.actions}>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => {
                setMode(isLogin ? "signup" : "login");
                setError(null);
              }}
            >
              {isLogin ? "Create account" : "Have an account? Sign in"}
            </Button>
            <Button
              type="submit"
              variant="primary"
              disabled={busy || !email.trim() || !password}
            >
              {busy ? "…" : isLogin ? "Sign in" : "Sign up"}
            </Button>
          </div>
        </form>
      </Panel>
    </div>
  );
}
