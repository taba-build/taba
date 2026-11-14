(function () {
  "use strict";
  /**
   * Sends a message to the backend.
   * @example
   * ```typescript
   * import { invoke } from '@tauri-apps/api/core';
   * await invoke('login', { user: 'tauri', password: 'poiwe3h4r5ip3yrhtew9ty' });
   * ```
   *
   * @param cmd The command name.
   * @param args The optional arguments to pass to the command.
   * @param options The request options.
   * @return A promise resolving or rejecting to the backend response.
   *
   * @since 1.0.0
   */
  async function invoke(cmd, args = {}, options) {
    return window.__TAURI_INTERNALS__.invoke(cmd, args, options);
  }
  //#endregion
  //#region packages/rpc/src/session.ts
  /** The `globalThis` key a backend sets the {@link Session} on. */
  const SESSION = "__taba_session__";
  //#endregion
  //#region crates/taba-tauri/src/injected.ts
  /** An app's `Err`. */
  var Failed = class {
    constructor(error) {
      this.error = error;
    }
  };
  const encoder = new TextEncoder();
  const outbox = [];
  const waiters = /* @__PURE__ */ new Map();
  let session;
  let locked = true;
  let nextId = 0;
  let failed;
  /** Rejects every pending and later request with `reason`. */
  function fail(reason) {
    failed ??= { reason };
    for (const waiter of waiters.values()) waiter.reject(reason);
    waiters.clear();
  }
  /** Batches `message` with whatever is queued when the next send starts. */
  function queue(message) {
    if (failed) return;
    outbox.push(message);
    if (!locked) {
      locked = true;
      queueMicrotask(() => flush().catch(fail));
    }
  }
  async function flush() {
    while (outbox.length > 0) await invoke("taba::send", encode(outbox.splice(0)));
    locked = false;
  }
  /** Skips and rejects any request `JSON.stringify` throws on. */
  function encode(batch) {
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
  function body(value) {
    return encoder.encode(JSON.stringify(value));
  }
  function idOf(message) {
    return message.kind === "call" || message.kind === "subscribe"
      ? message.value[0]
      : message.value;
  }
  function wait(message) {
    return new Promise((resolve, reject) => {
      if (failed) {
        reject(failed.reason);
        return;
      }
      waiters.set(idOf(message), {
        resolve,
        reject,
      });
      queue(message);
    });
  }
  async function call(name, request, { signal } = {}) {
    signal?.throwIfAborted();
    const id = nextId++;
    const cancel = () => {
      waiters.get(id)?.reject(signal?.reason);
      waiters.delete(id);
      queue({
        kind: "cancel",
        value: id,
      });
    };
    signal?.addEventListener("abort", cancel);
    try {
      return (
        await wait({
          kind: "call",
          value: [id, name, request],
        })
      ).value;
    } finally {
      signal?.removeEventListener("abort", cancel);
    }
  }
  async function* subscribe(name, request, signal) {
    signal.throwIfAborted();
    const id = nextId++;
    const cancel = () => {
      waiters.get(id)?.reject(signal.reason);
      waiters.delete(id);
      queue({
        kind: "cancel",
        value: id,
      });
    };
    signal.addEventListener("abort", cancel);
    queue({
      kind: "subscribe",
      value: [id, name, request],
    });
    try {
      for (;;) {
        signal.throwIfAborted();
        const result = await wait({
          kind: "next",
          value: id,
        });
        if (result.done) return;
        yield result.value;
        if (result.value instanceof Failed) return;
      }
    } finally {
      signal.removeEventListener("abort", cancel);
      if (!signal.aborted) cancel();
    }
  }
  async function tryCall(...args) {
    return outcome(await call(...args));
  }
  async function* trySubscribe(...args) {
    for await (const value of subscribe(...args)) yield outcome(value);
  }
  /** The one place a `{ ok, ... }` is built. */
  function outcome(value) {
    return value instanceof Failed
      ? {
          ok: false,
          error: value.error,
        }
      : {
          ok: true,
          value,
        };
  }
  /** Hands each frame to its waiter until a poll rejects. */
  async function polls() {
    for (;;) {
      const frame = await invoke("taba::poll", body(session));
      const waiter = waiters.get(frame.id);
      waiters.delete(frame.id);
      switch (frame.kind) {
        case "value":
          waiter?.resolve({
            done: false,
            value: frame.value,
          });
          break;
        case "error":
          waiter?.resolve({
            done: false,
            value: new Failed(frame.value),
          });
          break;
        case "end":
          waiter?.resolve({
            done: true,
            value: void 0,
          });
          break;
        case "reject":
          waiter?.reject(frame.value);
      }
    }
  }
  if (window.top === window) {
    const instance = Object.freeze({
      call,
      subscribe,
      tryCall,
      trySubscribe,
    });
    Object.defineProperty(globalThis, SESSION, { value: instance });
    invoke("taba::connect")
      .then((id) => {
        session = id;
        flush().catch(fail);
        return polls();
      })
      .catch(fail);
  }
  //#endregion
})();
