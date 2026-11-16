import type * as $ from "./taba.js";
export type $Output<Declared, Parts> = $Expanded<
  $OutputWithRest<Declared & $AllKnown<Parts>, $AllOutputRest<Parts>>
>;
export type $Input<Declared, Parts> = $Expanded<
  $InputWithRest<Declared & $AllKnown<Parts>, $AllInputRest<Parts>>
>;
export type $Expanded<T> = T extends unknown ? { [Key in keyof T]: T[Key] } : never;
export type $Flattened<Part> = Part extends null
  ? {}
  : Part extends string
    ? $UnitVariant<Part>
    : Part;
export type $UnitVariant<Name extends string> = string extends Name ? never : Record<Name, null>;
export type $AllKnown<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $Known<$Flattened<Part>> & $AllKnown<Tail>
  : {};
export type $Known<Part> = Part extends unknown
  ? { [Key in keyof Part as string extends Key ? never : Key]: Part[Key] }
  : never;
export type $OutputWithRest<Known, Rest> = [Rest] extends [never]
  ? Known
  : Known extends unknown
    ? { [Key in keyof Known]: Known[Key] | Rest } & {
        [key: string]: Rest | Known[keyof Known];
      }
    : never;
export type $AllOutputRest<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $OutputRest<$Flattened<Part>> | $AllOutputRest<Tail>
  : never;
export type $OutputRest<Part> = Part extends unknown
  ? string extends keyof Part
    ? Part[string]
    : never
  : never;
export type $InputWithRest<Known, Rest> = unknown extends Rest
  ? Known
  : Known extends {
        [key: string]: Rest;
      }
    ? Known & {
        [key: string]: Rest;
      }
    : Known;
export type $AllInputRest<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $InputRest<$Flattened<Part>> & $AllInputRest<Tail>
  : unknown;
export type $InputRest<Part> = Part extends unknown
  ? string extends keyof Part
    ? Part[string]
    : unknown
  : never;
export namespace crate.game {
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
      | $.crate.game.Generator.Input
      | {
          generator: $.crate.game.Generator.Input;
          /** How many beyond the first, none when absent. */
          extra?: number;
          /** A note for the shop. */
          note: $.serde_json.value.Value.Input;
        }
      | $.crate.game.Shipment.Input<$.crate.game.shop.Generator.Input>;
  }
  /** No change in the weather. */
  export type Calm = null;
  export namespace Calm {
    /** No change in the weather. */
    export type Input = null;
  }
  /** A cart of ore, by kind. */
  export type Cart = $Output<
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
    export type Input = $Input<
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
  export type Claim = $Output<
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
    export type Input = $Input<
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
  /** The next tick's forecast. */
  export type Forecast = $Output<
    {
      tick: number;
    },
    [$.crate.game.Calm, $.crate.game.Weather, $.crate.game.Boost | null]
  >;
  export namespace Forecast {
    /** The next tick's forecast. */
    export type Input = $Input<
      {
        tick: number;
      },
      [$.crate.game.Calm.Input, $.crate.game.Weather.Input, $.crate.game.Boost.Input | null]
    >;
  }
  /** A machine that makes ore. */
  export type Generator = "drill" | "furnace";
  export namespace Generator {
    /** A machine that makes ore. */
    export type Input = "drill" | "furnace";
  }
  /** A hold, and its ore by kind once it has some. */
  export type Hold = $Output<
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
    export type Input = $Input<
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
  export type Mine = $Output<
    {
      /** Ore on hand, by kind. */
      ore: {
        [key: string]: number;
      };
      /** The last hauls, newest first. */
      hauls: [$.crate.game.Generator, number | null][];
      /** What the shop sells. */
      for_sale: $.crate.game.shop.Generator[];
      /** The level below, dug on a `*\/5` schedule. */
      deeper: $.crate.game.Mine | null;
      running: boolean;
      /** The last survey, as the surveyor wrote it. */
      survey: $.serde_json.value.Value;
      "gift-note": string;
      /** What's on its way. */
      incoming: $.crate.game.Shipment<$.crate.game.shop.Generator>[];
      /** The generator that dug last. */
      last_digger: $.crate.game.Id;
    },
    [$.crate.game.Boost | null, $.crate.game.Weather]
  >;
  /** The surveyor's notes, by vein. */
  export type Notebook = $Output<
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
    export type Input = $Input<
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
  export type Parcel<T> = $Output<
    {
      weight: number;
    },
    [T]
  >;
  export namespace Parcel {
    /** A parcel, whatever it holds. */
    export type Input<T> = $Input<
      {
        weight: number;
      },
      [T]
    >;
  }
  /** Something on its way to the mine. */
  export type Shipment<T> = $Output<
    {
      /** Ticks until it arrives. */
      eta: number;
      /** The shipment after it. */
      next: $.crate.game.Shipment<T> | null;
    },
    [T]
  >;
  export namespace Shipment {
    /** Something on its way to the mine. */
    export type Input<T> = $Input<
      {
        /** Ticks until it arrives. */
        eta: number;
        /** The shipment after it. */
        next?: $.crate.game.Shipment.Input<T> | null;
      },
      [T]
    >;
  }
  /** Ore piled up under a boost, by kind. */
  export type Stockpile = $Output<
    {},
    [
      $.crate.game.Boost,
      {
        [key: string]: number;
      },
    ]
  >;
  export namespace Stockpile {
    /** Ore piled up under a boost, by kind. */
    export type Input = $Input<
      {},
      [
        $.crate.game.Boost.Input,
        {
          [key: string]: number;
        },
      ]
    >;
  }
  /** A train, and the wagon it pulls. */
  export type Train = $Output<
    {
      engine: string;
    },
    [$.crate.game.Wagon]
  >;
  export namespace Train {
    /** A train, and the wagon it pulls. */
    export type Input = $Input<
      {
        engine: string;
      },
      [$.crate.game.Wagon.Input]
    >;
  }
  export type Unreachable = never;
  /** A wagon of ore, by kind. */
  export type Wagon = $Output<
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
    export type Input = $Input<
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
export namespace crate.game.shop {
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
