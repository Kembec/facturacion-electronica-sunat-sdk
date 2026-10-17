package com.kembec.sunatsdk;

/** What every result keeps: the response exactly as the native library gave it. */
public abstract class SdkResult {
    private final String rawJson;

    SdkResult(String rawJson) {
        this.rawJson = rawJson;
    }

    /**
     * The JSON of docs/cli.md for this result, for logs and for members newer than this package.
     *
     * @return the response as the library gave it
     */
    public String getRawJson() {
        return rawJson;
    }
}
