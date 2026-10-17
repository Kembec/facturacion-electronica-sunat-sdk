package com.kembec.sunatsdk;

/**
 * A command that did not run: the SDK's own error (the CLI's {@code {"error"}} with exit code 1, such as an invalid
 * document, a missing certificate or a send without an answer) or a response that is not the contract's. An answer
 * from SUNAT, a rejection included, is a result, never this exception.
 */
public class SunatSdkException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    /** The command of docs/cli.md that failed; null if none ran. */
    private final String command;

    /**
     * Creates the exception for {@code command}.
     *
     * @param command the command of docs/cli.md that failed; null if none ran
     * @param message what went wrong
     */
    public SunatSdkException(String command, String message) {
        super(message);
        this.command = command;
    }

    /**
     * Creates the exception for {@code command} with its cause.
     *
     * @param command the command of docs/cli.md that failed; null if none ran
     * @param message what went wrong
     * @param cause the cause
     */
    public SunatSdkException(String command, String message, Throwable cause) {
        super(message, cause);
        this.command = command;
    }

    /**
     * The command that failed.
     *
     * @return {@code build}, {@code send}...; null if none ran
     */
    public String getCommand() {
        return command;
    }
}
