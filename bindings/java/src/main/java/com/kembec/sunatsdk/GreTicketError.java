package com.kembec.sunatsdk;

import java.util.Map;

/** The {@code error} of a GRE ticket: SUNAT's code and text, and its class. */
public final class GreTicketError {
    private final String code;
    private final String description;
    private final ResponseClass responseClass;

    GreTicketError(Map<String, Object> value) {
        code = Fields.optionalText(value, "code");
        description = Fields.text(value, "description");
        responseClass = ResponseClass.of(Fields.optionalText(value, "class"));
    }

    /**
     * SUNAT's {@code numError}.
     *
     * @return the code, or null
     */
    public String getCode() {
        return code;
    }

    /**
     * SUNAT's {@code desError}.
     *
     * @return the text
     */
    public String getDescription() {
        return description;
    }

    /**
     * How the SDK classifies the code.
     *
     * @return the class
     */
    public ResponseClass getResponseClass() {
        return responseClass;
    }
}
