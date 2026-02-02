/**
 * Parse a Styx document with a schema, returning typed JS values
 */
export declare function parseTyped<T = unknown>(source: string, schemaSource: string): T;
/**
 * Parse a Styx document without a schema, returning untyped JS values
 */
export declare function parseUntyped(source: string): unknown;
