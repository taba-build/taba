import { invoke } from "@tauri-apps/api/core";

import type { Result } from "#rpc/index.js";
import { SESSION, type Session } from "#rpc/session.js";
import type { crate, serde_json } from "#rpc/taba.js";

type Message = crate.rpc.wire.Message.Input;

/** An app's `Err`. */
class Failed {
  readonly error: serde_json.value.Value;

  constructor(error: serde_json.value.Value) {
    this.error = error;
  }
}

type Answer = serde_json.value.Value | Failed;

type Waiter = {
  resolve: (result: IteratorResult<Answer, undefined>) => void;
  reject: (reason: unknown) => void;
};

const encoder = new TextEncoder();
const outbox: Message[] = [];
const waiters = new Map<number, Waiter>();
let session: crate.rpc.sessions.SessionId;
let locked = true; // nothing sends until connect answers
let nextId = 0;
let failed: { reason: unknown } | undefined;

/** Rejects every pending and later request with `reason`. */
function fail(reason: unknown) {
  failed ??= { reason };
  for (const waiter of waiters.values()) waiter.reject(reason);
  waiters.clear();
}

/** Batches `message` with whatever is queued when the next send starts. */
function queue(message: Message) {
  if (failed) return;
  outbox.push(message);
  if (!locked) {
    locked = true;
    queueMicrotask(() => flush().catch(fail));
  }
}

async function flush() {
  // one send at a time keeps the messages in order
  while (outbox.length > 0) await invoke("taba::send", encode(outbox.splice(0)));
  locked = false;
}

/** Skips and rejects any request `JSON.stringify` throws on. */
function encode(batch: Message[]): Uint8Array {
  try {
    return body([session, batch]);
  } catch {
    const good = batch.filter((message) => {
      try {
        JSON.stringify(message);
        return true;
      } catch (reason) {
        const id = idOf(message);
        waiters.get(id)?.reject(reason);
        waiters.delete(id);
        return false;
      }
    });
    return body([session, good]);
  }
}

function body(value: crate.rpc.wire.Batch.Input | crate.rpc.sessions.SessionId.Input): Uint8Array {
  return encoder.encode(JSON.stringify(value));
}

function idOf(message: Message): number {
  return message.kind === "call" || message.kind === "subscribe" ? message.value[0] : message.value;
}

function wait(message: Message): Promise<IteratorResult<Answer, undefined>> {
  return new Promise((resolve, reject) => {
    if (failed) {
      reject(failed.reason);
      return;
    }
    waiters.set(idOf(message), { resolve, reject });
    queue(message);
  });
}

async function call<T>(
  name: string,
  request: serde_json.value.Value.Input,
  { signal }: { signal?: AbortSignal } = {},
): Promise<T> {
  signal?.throwIfAborted();
  const id = nextId++;
  const cancel = () => {
    waiters.get(id)?.reject(signal?.reason);
    waiters.delete(id);
    queue({ kind: "cancel", value: id });
  };
  signal?.addEventListener("abort", cancel);
  try {
    const result = await wait({ kind: "call", value: [id, name, request] });
    return result.value as T;
  } finally {
    signal?.removeEventListener("abort", cancel);
  }
}

async function* subscribe<T>(
  name: string,
  request: serde_json.value.Value.Input,
  signal: AbortSignal,
): AsyncGenerator<T, void, undefined> {
  signal.throwIfAborted();
  const id = nextId++;
  const cancel = () => {
    waiters.get(id)?.reject(signal.reason);
    waiters.delete(id);
    queue({ kind: "cancel", value: id });
  };
  signal.addEventListener("abort", cancel);
  queue({ kind: "subscribe", value: [id, name, request] });
  try {
    for (;;) {
      signal.throwIfAborted();
      const result = await wait({ kind: "next", value: id });
      if (result.done) return;
      yield result.value as T;
      if (result.value instanceof Failed) return; // an app's error is a stream's last item
    }
  } finally {
    signal.removeEventListener("abort", cancel);
    if (!signal.aborted) cancel(); // an abort already cancelled
  }
}

async function tryCall<T, E>(...args: Parameters<Session["tryCall"]>): Promise<Result<T, E>> {
  return outcome<T, E>(await call<Answer>(...args));
}

async function* trySubscribe<T, E>(
  ...args: Parameters<Session["trySubscribe"]>
): AsyncGenerator<Result<T, E>, void, undefined> {
  for await (const value of subscribe<Answer>(...args)) yield outcome<T, E>(value);
}

/** The one place a `{ ok, ... }` is built. */
function outcome<T, E>(value: Answer): Result<T, E> {
  return value instanceof Failed
    ? { ok: false, error: value.error as E }
    : { ok: true, value: value as T };
}

/** Hands each frame to its waiter until a poll rejects. */
async function polls() {
  for (;;) {
    const frame = await invoke<crate.rpc.wire.Frame>("taba::poll", body(session));
    const waiter = waiters.get(frame.id);
    waiters.delete(frame.id);
    switch (frame.kind) {
      case "value":
        waiter?.resolve({ done: false, value: frame.value });
        break;
      case "error":
        waiter?.resolve({ done: false, value: new Failed(frame.value) });
        break;
      case "end":
        waiter?.resolve({ done: true, value: undefined });
        break;
      case "reject":
        waiter?.reject(frame.value);
        break;
    }
  }
}

// main frame only, preventing iframes from affecting the page's session
if (window.top === window) {
  const instance = Object.freeze({ call, subscribe, tryCall, trySubscribe }) satisfies Session;
  Object.defineProperty(globalThis, SESSION, { value: instance });
  invoke<crate.rpc.sessions.SessionId>("taba::connect")
    .then((id) => {
      session = id;
      flush().catch(fail);
      return polls();
    })
    .catch(fail);
}
