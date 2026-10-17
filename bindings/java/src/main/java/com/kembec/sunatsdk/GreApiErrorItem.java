package com.kembec.sunatsdk;

/** One validation error of a 422 from the GRE API. */
public final class GreApiErrorItem {
    private final String code;
    private final String description;

    GreApiErrorItem(String code, String description) {
        this.code = code;
        this.description = description;
    }

    /**
     * SUNAT's code.
     *
     * @return the code, or null
     */
    public String getCode() {
        return code;
    }

    /**
     * SUNAT's text.
     *
     * @return the text
     */
    public String getDescription() {
        return description;
    }
}
