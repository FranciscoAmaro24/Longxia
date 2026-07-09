/**
 * Transport for the core API. The same typed wrappers in `api.ts` run on two
 * hosts: inside the Tauri webview each call goes through `invoke`; in a plain
 * browser it becomes a `fetch` to `longxia-server`. Detection is by the Tauri
 * internals the webview injects, so the browser build never calls `invoke`.
 *
 * Each wrapper declares both forms once (a `CallSpec`), so the TS/Rust contract
 * still lives in a single place regardless of which host is running.
 */
import { invoke } from "@tauri-apps/api/core";

const IS_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Same-origin by default: in production the server serves the built SPA, and in
// web dev a Vite proxy forwards `/api` to the server (see vite.config.ts), so no
// CORS is needed. Override at build time with VITE_API_BASE for a split deploy.
const API_BASE = String(import.meta.env.VITE_API_BASE ?? "").replace(/\/$/, "");

// Bearer token for the (browser) HTTP transport. `longxia-server` requires it on
// every route except health. It is not baked into the bundle: the user provides
// it at runtime via `setApiToken` (stored in localStorage), with an optional
// build-time VITE_API_TOKEN fallback for a trusted single-tenant deploy. The
// Tauri host never uses this - it calls the local core directly, with no token.
const TOKEN_KEY = "longxia_token";

/** Store (or clear, with `null`) the API token used by the browser transport. */
export function setApiToken(token: string | null): void {
  try {
    if (token) {
      window.localStorage.setItem(TOKEN_KEY, token);
    } else {
      window.localStorage.removeItem(TOKEN_KEY);
    }
  } catch {
    // localStorage unavailable (private mode, etc.); token just will not persist.
  }
}

function authToken(): string | null {
  try {
    const stored = window.localStorage.getItem(TOKEN_KEY);
    if (stored) return stored;
  } catch {
    // ignore and fall through to the build-time fallback
  }
  const baked = import.meta.env.VITE_API_TOKEN;
  return typeof baked === "string" && baked ? baked : null;
}

/** Whether a token is available for the browser transport (or none is needed). */
export function hasApiToken(): boolean {
  return authToken() !== null;
}

export type HttpMethod = "GET" | "POST" | "PUT" | "DELETE";

export interface HttpSpec {
  method: HttpMethod;
  /** Path under the API base, e.g. "/api/today". */
  path: string;
  query?: Record<string, string>;
  body?: unknown;
}

export interface CallSpec<T> {
  /** Tauri command name. */
  command: string;
  /** Named args for the Tauri command. */
  args?: Record<string, unknown>;
  /** How the same call is made over HTTP in the browser. */
  http: HttpSpec;
  /** Adapt the HTTP JSON to the command's return shape, when they differ. */
  fromHttp?: (raw: unknown) => T;
}

/** Whether the code is running inside the Tauri webview. */
export function isTauri(): boolean {
  return IS_TAURI;
}

export async function call<T>(spec: CallSpec<T>): Promise<T> {
  if (IS_TAURI) {
    return invoke<T>(spec.command, spec.args);
  }
  return httpCall(spec);
}

async function httpCall<T>(spec: CallSpec<T>): Promise<T> {
  const { method, path, query, body } = spec.http;
  const qs = query ? "?" + new URLSearchParams(query).toString() : "";

  const headers: Record<string, string> = {};
  if (body !== undefined) headers["content-type"] = "application/json";
  const token = authToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;

  const res = await fetch(API_BASE + path + qs, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (!res.ok) {
    // A stored token the server rejects is wrong or expired: clear it so the
    // token gate is shown again on reload rather than looping on 401s.
    if (res.status === 401) setApiToken(null);
    // Match the Tauri contract, which rejects with the plain message string, so
    // call sites (`String(e)`) render the error the same way on both hosts.
    throw await errorMessage(res);
  }
  if (res.status === 204) {
    return undefined as T;
  }
  const raw: unknown = await res.json();
  return spec.fromHttp ? spec.fromHttp(raw) : (raw as T);
}

// --- Accounts (web transport only; the Tauri app talks to the local core) ---

export interface AuthResult {
  email: string;
}

async function authRequest(
  path: string,
  body: Record<string, unknown>,
): Promise<AuthResult> {
  const res = await fetch(API_BASE + path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw await errorMessage(res);
  }
  const data = (await res.json()) as { token: string; email: string };
  setApiToken(data.token); // subsequent requests carry this session token
  return { email: data.email };
}

export function signup(
  email: string,
  password: string,
  invite?: string,
): Promise<AuthResult> {
  return authRequest("/api/auth/signup", { email, password, invite: invite || undefined });
}

export function login(email: string, password: string): Promise<AuthResult> {
  return authRequest("/api/auth/login", { email, password });
}

/** End the current session on the server, then clear the local token. */
export async function logout(): Promise<void> {
  const token = authToken();
  try {
    await fetch(API_BASE + "/api/auth/logout", {
      method: "POST",
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    });
  } catch {
    // Best effort; clear locally regardless.
  }
  setApiToken(null);
}

async function errorMessage(res: Response): Promise<string> {
  try {
    const body = (await res.json()) as { error?: unknown };
    if (body && typeof body.error === "string") {
      return body.error;
    }
  } catch {
    // Non-JSON body; fall through to a generic message.
  }
  return `Request failed (${res.status})`;
}
