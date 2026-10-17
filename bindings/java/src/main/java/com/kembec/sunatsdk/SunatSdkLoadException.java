package com.kembec.sunatsdk;

/**
 * The native library was not found for this platform, could not be loaded, or comes from a version of the SDK with
 * another ABI. Nothing ran.
 */
public final class SunatSdkLoadException extends SunatSdkException {
    private static final long serialVersionUID = 1L;

    /**
     * Creates the exception.
     *
     * @param message what went wrong
     */
    public SunatSdkLoadException(String message) {
        super(null, message);
    }

    /**
     * Creates the exception with its cause.
     *
     * @param message what went wrong
     * @param cause the cause
     */
    public SunatSdkLoadException(String message, Throwable cause) {
        super(null, message, cause);
    }
}
