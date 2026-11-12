import { session, type Session } from "./session.js";

/** What a fallible procedure resolves to. */
export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

/** Calls the procedure `name`. */
export const call: Session["call"] = (...args) => session().call(...args);

/** Calls the fallible procedure `name`. */
export const tryCall: Session["tryCall"] = (...args) => session().tryCall(...args);

/** Subscribes to the procedure `name`, until `signal` aborts or its stream ends. */
export const subscribe: Session["subscribe"] = (...args) => session().subscribe(...args);

/** Subscribes to the fallible procedure `name`, until `signal` aborts or its stream ends. */
export const trySubscribe: Session["trySubscribe"] = (...args) => session().trySubscribe(...args);
