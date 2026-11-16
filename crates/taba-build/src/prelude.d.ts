/** The keys `T` adds where serde flattens it. */
type $Flattened<T> = T extends null ? {} : T extends string ? $UnitVariant<T> : T;

/** A unit variant `"A"` writes `{ "A": null }`, and a plain string was never a variant. */
type $UnitVariant<Name extends string> = string extends Name ? never : { [Variant in Name]: null };
