import type * as $ from "./taba.js";
export namespace serde_json.value {
  export type Value =
    | null
    | boolean
    | number
    | string
    | $.serde_json.value.Value[]
    | {
        [key: string]: $.serde_json.value.Value;
      };
  export namespace Value {
    export type Input =
      | null
      | boolean
      | number
      | string
      | $.serde_json.value.Value.Input[]
      | {
          [key: string]: $.serde_json.value.Value.Input;
        };
  }
}
export namespace crate.rpc.sessions {
  /** A session's id. */
  export type SessionId = number;
  export namespace SessionId {
    /** A session's id. */
    export type Input = number;
  }
}
export namespace crate.rpc.wire {
  export namespace Batch {
    /** A session's [`Message`]s, in the order TypeScript sent them. */
    export type Input = [$.crate.rpc.sessions.SessionId.Input, $.crate.rpc.wire.Message.Input[]];
  }
  /** What a poll returns. */
  export type Frame = {
    /** Its message's id. */
    id: number;
  } & $.crate.rpc.wire.Kind;
  /** What a frame tells TypeScript. */
  export type Kind =
    | {
        kind: "value";
        value: $.serde_json.value.Value;
      }
    | {
        kind: "error";
        value: $.serde_json.value.Value;
      }
    | {
        kind: "end";
      }
    | {
        kind: "reject";
        value: $.crate.rpc.wire.Rejection;
      };
  export namespace Message {
    /**
     * Requests from TypeScript.
     *
     * `value` comes last for [`RawValue`]'s deferred parsing.
     */
    export type Input =
      | {
          kind: "call";
          value: [number, string, $.serde_json.value.Value.Input];
        }
      | {
          kind: "subscribe";
          value: [number, string, $.serde_json.value.Value.Input];
        }
      | {
          kind: "next";
          value: number;
        }
      | {
          kind: "cancel";
          value: number;
        };
  }
  /** Why [`taba`](crate) rejected a request. */
  export type Rejection =
    | {
        kind: "unknown";
        value: string;
      }
    | {
        kind: "invalid";
        value: string;
      }
    | {
        kind: "unencodable";
        value: string;
      }
    | {
        kind: "panic";
        value: string;
      };
}
