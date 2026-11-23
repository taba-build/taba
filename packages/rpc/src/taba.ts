import type * as $ from "./taba.js";
type $OutputWithParts<Declared, Parts> = $Expanded<
  $OutputWithRest<Declared & $AllKnown<Parts>, $AllOutputRest<Parts>>
>;
type $Expanded<T> = T extends unknown ? { [Key in keyof T]: T[Key] } : never;
type $Flattened<Part> = Part extends null ? {} : Part extends string ? $UnitVariant<Part> : Part;
type $UnitVariant<Name extends string> = string extends Name ? never : Record<Name, null>;
type $AllKnown<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $Known<$Flattened<Part>> & $AllKnown<Tail>
  : {};
type $Known<Part> = Part extends unknown
  ? { [Key in keyof Part as string extends Key ? never : Key]: Part[Key] }
  : never;
type $OutputWithRest<Known, Rest> = [Rest] extends [never]
  ? Known
  : Known extends unknown
    ? { [Key in keyof Known]: Known[Key] | Rest } & {
        [key: string]: Rest | Known[keyof Known];
      }
    : never;
type $AllOutputRest<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $OutputRest<$Flattened<Part>> | $AllOutputRest<Tail>
  : never;
type $OutputRest<Part> = Part extends unknown
  ? string extends keyof Part
    ? Part[string]
    : never
  : never;
declare namespace serde_json.value {
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
declare namespace taba.rpc.sessions {
  /** A session's id. */
  export type SessionId = number;
  export namespace SessionId {
    /** A session's id. */
    export type Input = number;
  }
}
declare namespace taba.rpc.wire {
  export namespace Batch {
    /** A session's [`Message`]s, in the order TypeScript sent them. */
    export type Input = [$.taba.rpc.sessions.SessionId.Input, $.taba.rpc.wire.Message.Input[]];
  }
  /** What a poll returns. */
  export type Frame = $OutputWithParts<
    {
      /** Its message's id. */
      id: number;
    },
    [$.taba.rpc.wire.Kind]
  >;
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
        value: $.taba.rpc.wire.Rejection;
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
export type { serde_json, taba };
