// A field with `#[serde(flatten)]` merges its keys into the object around it, and those keys can
// depend on a type parameter. So taba prints the object as `$Output<Declared, Parts>` or
// `$Input<Declared, Parts>`, and these aliases work out its keys in TypeScript wherever the type
// is used.
//
// Why not intersect the declared keys with each part? A flattened map takes every key left over,
// and an index signature can't leave the declared keys out. With a map part, `$Output` accepts
// more than serde writes and `$Input` less than serde reads.

/** The `Declared` keys and every part's keys, as serde writes them. */
export type $Output<Declared, Parts> = $Expanded<
  $OutputWithRest<Declared & $AllKnown<Parts>, $AllOutputRest<Parts>>
>;

/** The `Declared` keys and every part's keys, as serde reads them. */
export type $Input<Declared, Parts> = $Expanded<
  $InputWithRest<Declared & $AllKnown<Parts>, $AllInputRest<Parts>>
>;

/** `T` expanded into one object type, so hovers and errors show its keys, not intersections. */
export type $Expanded<T> = T extends unknown ? { [Key in keyof T]: T[Key] } : never;

/** The keys a part adds. `None` and a unit struct are `null` and add none. */
export type $Flattened<Part> = Part extends null
  ? {}
  : Part extends string
    ? $UnitVariant<Part>
    : Part;

/** The key a unit variant `"A"` adds, `{ A: null }`. serde can't flatten a plain string. */
export type $UnitVariant<Name extends string> = string extends Name ? never : Record<Name, null>;

/** Every part's known keys, which serde takes before a map sees the rest. */
export type $AllKnown<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $Known<$Flattened<Part>> & $AllKnown<Tail>
  : {};

/**
 * A part's known keys, without its index signature.
 *
 * Keep `Part extends unknown` so errors print the keys rather than `$Known<...>`.
 */
export type $Known<Part> = Part extends unknown
  ? { [Key in keyof Part as string extends Key ? never : Key]: Part[Key] }
  : never;

/**
 * The known keys, with every map's entries beside them.
 *
 * A map entry can repeat a known key, and `JSON.parse` keeps the last. Keep `Known extends
 * unknown`, so a union keeps each member's keys rather than only the ones they share.
 */
export type $OutputWithRest<Known, Rest> = [Rest] extends [never]
  ? Known
  : Known extends unknown
    ? { [Key in keyof Known]: Known[Key] | Rest } & { [key: string]: Rest | Known[keyof Known] }
    : never;

/** What any map writes. */
export type $AllOutputRest<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $OutputRest<$Flattened<Part>> | $AllOutputRest<Tail>
  : never;

/** What a part writes beside the known keys, nothing unless it's a map. */
export type $OutputRest<Part> = Part extends unknown
  ? string extends keyof Part
    ? Part[string]
    : never
  : never;

/**
 * The known keys, and other keys if every known key fits the rest too.
 *
 * TypeScript can type "every key" but not "every other key".
 */
export type $InputWithRest<Known, Rest> = unknown extends Rest
  ? Known
  : Known extends { [key: string]: Rest }
    ? Known & { [key: string]: Rest }
    : Known;

/** What every map accepts, since each one sees every key left over. */
export type $AllInputRest<Parts> = Parts extends [infer Part, ...infer Tail]
  ? $InputRest<$Flattened<Part>> & $AllInputRest<Tail>
  : unknown;

/** What a part accepts for the keys left over, anything unless it's a map. */
export type $InputRest<Part> = Part extends unknown
  ? string extends keyof Part
    ? Part[string]
    : unknown
  : never;
