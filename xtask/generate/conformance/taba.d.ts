import type * as $ from "./taba.js";
type $OutputWithParts<Declared, Parts> = $Expanded<
  $OutputWithRest<Declared & $AllKnown<Parts>, $AllOutputRest<Parts>>
>;
type $InputWithParts<Declared, Parts> = $Expanded<
  $InputWithRest<Declared & $AllKnown<Parts>, $AllInputRest<Parts>>
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
type $InputWithRest<Known, Rest> = unknown extends Rest
  ? Known
  : Known extends {
        [key: string]: Rest;
      }
    ? Known & {
        [key: string]: Rest;
      }
    : Known;
type $AllInputRest<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $InputRest<$Flattened<Part>> & $AllInputRest<Tail>
  : unknown;
type $InputRest<Part> = Part extends unknown
  ? string extends keyof Part
    ? Part[string]
    : unknown
  : never;
declare namespace generate.game {
  /** A boost the mine is under. */
  export type Boost = {
    /** How many times faster it digs. */
    rate: number;
  };
  export namespace Boost {
    /** A boost the mine is under. */
    export type Input = {
      /** How many times faster it digs. */
      rate: number;
    };
  }
  export namespace Buy {
    /** Buys generators. */
    export type Input =
      | $.generate.game.Generator.Input
      | {
          generator: $.generate.game.Generator.Input;
          /** How many beyond the first, none when absent. */
          extra?: number;
          /** A note for the shop. */
          note: $.serde_json.value.Value.Input;
        }
      | $.generate.game.Shipment.Input<$.generate.game.shop.Generator.Input>;
  }
  /** No change in the weather. */
  export type Calm = null;
  export namespace Calm {
    /** No change in the weather. */
    export type Input = null;
  }
  /** A cart of ore, by kind. */
  export type Cart = $OutputWithParts<
    {
      /** What pulls it. */
      pulled_by: string;
    },
    [
      {
        [key: string]: number;
      },
    ]
  >;
  export namespace Cart {
    /** A cart of ore, by kind. */
    export type Input = $InputWithParts<
      {
        /** What pulls it. */
        pulled_by: string;
      },
      [
        {
          [key: string]: number;
        },
      ]
    >;
  }
  /** A claim on a vein. */
  export type Claim = $OutputWithParts<
    {
      owner: string;
    },
    [
      {
        [key: string]: number;
      },
      {
        [key: string]: string;
      },
    ]
  >;
  export namespace Claim {
    /** A claim on a vein. */
    export type Input = $InputWithParts<
      {
        owner: string;
      },
      [
        {
          [key: string]: number;
        },
        {
          [key: string]: string;
        },
      ]
    >;
  }
  /** Something that happened at the mine. */
  export type Event =
    | $OutputWithParts<
        {
          kind: "Bought";
        },
        [$.generate.game.shop.Generator]
      >
    | $OutputWithParts<
        {
          kind: "Sold";
        },
        [
          {
            [key: string]: number;
          },
        ]
      >
    | $OutputWithParts<
        {
          kind: "Calmed";
        },
        [$.generate.game.Calm]
      >
    | $OutputWithParts<
        {
          kind: "Changed";
        },
        [$.generate.game.Weather]
      >;
  export namespace Event {
    /** Something that happened at the mine. */
    export type Input =
      | $InputWithParts<
          {
            kind: "Bought";
          },
          [$.generate.game.shop.Generator.Input]
        >
      | $InputWithParts<
          {
            kind: "Sold";
          },
          [
            {
              [key: string]: number;
            },
          ]
        >
      | $InputWithParts<
          {
            kind: "Calmed";
          },
          [$.generate.game.Calm.Input]
        >
      | $InputWithParts<
          {
            kind: "Changed";
          },
          [$.generate.game.Weather.Input]
        >
      | $InputWithParts<
          {
            kind: "Boosted";
          },
          [$.generate.game.Boost.Input]
        >
      | $InputWithParts<
          {
            kind: "Faded";
          },
          [$.generate.game.Boost.Input]
        >
      | $InputWithParts<
          {
            kind: "Stored";
          },
          [$.generate.game.Boost.Input]
        >;
  }
  /** The next tick's forecast. */
  export type Forecast = $OutputWithParts<
    {
      tick: number;
    },
    [$.generate.game.Calm, $.generate.game.Weather, $.generate.game.Boost | null]
  >;
  export namespace Forecast {
    /** The next tick's forecast. */
    export type Input = $InputWithParts<
      {
        tick: number;
      },
      [
        $.generate.game.Calm.Input,
        $.generate.game.Weather.Input,
        $.generate.game.Boost.Input | null,
      ]
    >;
  }
  /** A machine that makes ore. */
  export type Generator = "drill" | "furnace";
  export namespace Generator {
    /** A machine that makes ore. */
    export type Input = "drill" | "furnace";
  }
  /** A hold, and its ore by kind once it has some. */
  export type Hold = $OutputWithParts<
    {
      owner: string;
    },
    [
      {
        [key: string]: number;
      } | null,
    ]
  >;
  export namespace Hold {
    /** A hold, and its ore by kind once it has some. */
    export type Input = $InputWithParts<
      {
        owner: string;
      },
      [
        {
          [key: string]: number;
        } | null,
      ]
    >;
  }
  /** An id for one kind of thing. */
  export type Id = {
    raw: number;
    kind: null;
  };
  /**
   * The mine, after a tick.
   *
   * Sent once per tick.
   */
  export type Mine = $OutputWithParts<
    {
      /** Ore on hand, by kind. */
      ore: {
        [key: string]: number;
      };
      /** The last hauls, newest first. */
      hauls: [$.generate.game.Generator, number | null][];
      /** What the shop sells. */
      for_sale: $.generate.game.shop.Generator[];
      /** The level below, dug on a `*\/5` schedule. */
      deeper: $.generate.game.Mine | null;
      running: boolean;
      /** The last survey, as the surveyor wrote it. */
      survey: $.serde_json.value.Value;
      "gift-note": string;
      /** What's on its way. */
      incoming: $.generate.game.Shipment<$.generate.game.shop.Generator>[];
      /** The generator that dug last. */
      last_digger: $.generate.game.Id;
    },
    [$.generate.game.Boost | null, $.generate.game.Weather]
  >;
  /** The surveyor's notes, by vein. */
  export type Notebook = $OutputWithParts<
    {
      author: string;
    },
    [
      {
        [key: string]: $.serde_json.value.Value;
      },
    ]
  >;
  export namespace Notebook {
    /** The surveyor's notes, by vein. */
    export type Input = $InputWithParts<
      {
        author: string;
      },
      [
        {
          [key: string]: $.serde_json.value.Value.Input;
        },
      ]
    >;
  }
  /** A parcel, whatever it holds. */
  export type Parcel<T> = $OutputWithParts<
    {
      weight: number;
    },
    [T]
  >;
  export namespace Parcel {
    /** A parcel, whatever it holds. */
    export type Input<T> = $InputWithParts<
      {
        weight: number;
      },
      [T]
    >;
  }
  /** Something on its way to the mine. */
  export type Shipment<T> = $OutputWithParts<
    {
      /** Ticks until it arrives. */
      eta: number;
      /** The shipment after it. */
      next: $.generate.game.Shipment<T> | null;
    },
    [T]
  >;
  export namespace Shipment {
    /** Something on its way to the mine. */
    export type Input<T> = $InputWithParts<
      {
        /** Ticks until it arrives. */
        eta: number;
        /** The shipment after it. */
        next?: $.generate.game.Shipment.Input<T> | null;
      },
      [T]
    >;
  }
  /** Ore piled up under a boost, by kind. */
  export type Stockpile = $OutputWithParts<
    {},
    [
      $.generate.game.Boost,
      {
        [key: string]: number;
      },
    ]
  >;
  export namespace Stockpile {
    /** Ore piled up under a boost, by kind. */
    export type Input = $InputWithParts<
      {},
      [
        $.generate.game.Boost.Input,
        {
          [key: string]: number;
        },
      ]
    >;
  }
  /** Ore by kind, whatever names the kind. */
  export type Tally<K> = {
    [key: string]: number;
  };
  /** A train, and the wagon it pulls. */
  export type Train = $OutputWithParts<
    {
      engine: string;
    },
    [$.generate.game.Wagon]
  >;
  export namespace Train {
    /** A train, and the wagon it pulls. */
    export type Input = $InputWithParts<
      {
        engine: string;
      },
      [$.generate.game.Wagon.Input]
    >;
  }
  export type Unreachable = never;
  /** A wagon of ore, by kind. */
  export type Wagon = $OutputWithParts<
    {
      load: number;
    },
    [
      {
        [key: string]: number;
      },
    ]
  >;
  export namespace Wagon {
    /** A wagon of ore, by kind. */
    export type Input = $InputWithParts<
      {
        load: number;
      },
      [
        {
          [key: string]: number;
        },
      ]
    >;
  }
  /** The weather over the mine. */
  export type Weather =
    | "Clear"
    | {
        Storm: {
          strength: number;
        };
      };
  export namespace Weather {
    /** The weather over the mine. */
    export type Input =
      | "Clear"
      | {
          Storm: {
            strength: number;
          };
        };
  }
}
declare namespace generate.game.shop {
  /** A generator for sale. */
  export type Generator = {
    /** What it costs, in ore. */
    price: number;
  };
  export namespace Generator {
    /** A generator for sale. */
    export type Input = {
      /** What it costs, in ore. */
      price: number;
    };
  }
}
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
export type { generate, serde_json };
