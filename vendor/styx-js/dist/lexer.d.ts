import { Span } from "./types.js";
export type TokenType = "scalar" | "quoted" | "raw" | "heredoc" | "lbrace" | "rbrace" | "lparen" | "rparen" | "comma" | "at" | "tag" | "gt" | "newline" | "eof";
export interface Token {
    type: TokenType;
    text: string;
    span: Span;
    /** True if there was whitespace before this token */
    hadWhitespaceBefore: boolean;
    /** True if there was a newline before this token */
    hadNewlineBefore: boolean;
}
export declare class Lexer {
    private source;
    private pos;
    private bytePos;
    private line;
    private col;
    constructor(source: string);
    private peek;
    private advance;
    private utf8ByteLength;
    /** Get current byte position for span start */
    private get byteStart();
    private skipWhitespaceAndComments;
    nextToken(): Token;
    private isTagStart;
    private isTagChar;
    private readQuotedString;
    private readUnicodeEscape;
    private readRawString;
    private readHeredoc;
    /** Strip up to indentLen whitespace characters from the start of each line. */
    private dedentHeredoc;
    private readBareScalar;
}
