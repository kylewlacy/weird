import { Document, Span } from "./types.js";
export declare function documentToSexp(doc: Document): string;
export declare function errorToSexp(message: string, span: Span): string;
