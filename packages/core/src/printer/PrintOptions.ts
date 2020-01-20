/** Options for printing. */
export interface PrintOptions {
    /** The width the printer will attempt to keep the line under. */
    maxWidth: number;
    /** The number of spaces to use when indenting (unless useTabs is true). */
    indentWidth: number;
    /** Whether to use tabs for indenting. */
    useTabs: boolean;
    /** The newline character to use when doing a new line. */
    newLineKind: "\r\n" | "\n";
    /**
     * Set to true when testing in order to run additional validation on the inputted strings, which
     * ensures the printer is being used correctly.
     */
    isTesting: boolean;
}