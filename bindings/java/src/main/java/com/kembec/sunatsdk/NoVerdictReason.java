package com.kembec.sunatsdk;

/** Why there is no verdict. */
public enum NoVerdictReason {
    /** The wait ran out. */
    TIMEOUT("timeout"),
    /** SUNAT answered an HTTP status that says nothing about the document (401 included). */
    HTTP_STATUS("http_status");

    private final String wireName;

    NoVerdictReason(String wireName) {
        this.wireName = wireName;
    }

    static NoVerdictReason of(String name) {
        for (NoVerdictReason value : values()) {
            if (value.wireName.equals(name)) {
                return value;
            }
        }
        throw new Fields.ShapeError("unknown no_verdict reason `" + name + "`");
    }
}
