import type { Result } from "./index.js";
import type { serde_json } from "./taba.js";

/** The `globalThis` key a backend sets the {@link Session} on. */
export const SESSION = "__taba_session__";

/** The page's half of a taba session. */
export type Session = {
  call: <T>(
    name: string,
    request: serde_json.value.Value.Input,
    options?: { signal?: AbortSignal },
  ) => Promise<T>;

  tryCall: <T, E>(
    name: string,
    request: serde_json.value.Value.Input,
    options?: { signal?: AbortSignal },
  ) => Promise<Result<T, E>>;

  subscribe: <T>(
    name: string,
    request: serde_json.value.Value.Input,
    signal: AbortSignal,
  ) => AsyncGenerator<T, void, undefined>;

  trySubscribe: <T, E>(
    name: string,
    request: serde_json.value.Value.Input,
    signal: AbortSignal,
  ) => AsyncGenerator<Result<T, E>, void, undefined>;
};

/**
 * The {@link Session}, throwing if there's none.
 *
 * Read on every call, so importing doesn't need one.
 */
export function session(): Session {
  const instance = (globalThis as { [SESSION]?: Session })[SESSION];
  if (!instance) throw new Error("taba can't find its session");
  return instance;
}
