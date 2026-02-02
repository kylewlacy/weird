import { Document } from "./types.js";
export declare class Parser {
    private lexer;
    private source;
    private current;
    private peeked;
    constructor(source: string);
    private advance;
    private peek;
    private check;
    private expect;
    parse(): Document;
    private parseEntryWithPathCheck;
    private expandDottedPathWithState;
    private parseEntryWithDupCheck;
    private getKeyText;
    private validateKey;
    /** Get the span of just the heredoc opening marker (<<TAG\n). */
    private heredocStartSpan;
    private expandDottedPath;
    private parseAttributeValue;
    private parseTagValue;
    private parseValue;
    private parseAttributesStartingWith;
    private parseAttributesAfterGT;
    private parseScalar;
    private parseObject;
    private parseSequence;
}
export declare function parse(source: string): Document;
