/** Byte offset span in source */
export interface Span {
    start: number;
    end: number;
}
/** Scalar kinds */
export type ScalarKind = "bare" | "quoted" | "raw" | "heredoc";
/** Path value kind for tracking */
export type PathValueKind = "object" | "terminal";
/** Track path state for detecting reopen-path and nest-into-terminal errors */
export declare class PathState {
    currentPath: string[];
    closedPaths: Set<string>;
    assignedPaths: Map<string, {
        kind: PathValueKind;
        span: Span;
    }>;
    checkAndUpdate(path: string[], span: Span, kind: PathValueKind): void;
}
/** A scalar value */
export interface Scalar {
    type: "scalar";
    text: string;
    kind: ScalarKind;
    span: Span;
}
/** A sequence of values */
export interface Sequence {
    type: "sequence";
    items: Value[];
    span: Span;
}
/** An object entry */
export interface Entry {
    key: Value;
    value: Value;
}
/** An object (key-value pairs) */
export interface StyxObject {
    type: "object";
    entries: Entry[];
    span: Span;
}
/** A tag on a value */
export interface Tag {
    name: string;
    span: Span;
}
/** A Styx value */
export interface Value {
    tag?: Tag;
    payload?: Scalar | Sequence | StyxObject;
    span: Span;
}
/** Parse result */
export interface Document {
    entries: Entry[];
    span: Span;
}
/** Parse error */
export declare class ParseError extends Error {
    span: Span;
    constructor(message: string, span: Span);
}
