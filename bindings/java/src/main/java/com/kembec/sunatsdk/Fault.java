package com.kembec.sunatsdk;

import java.util.Map;

/** A SOAP fault: SUNAT refused the call with a code. */
public final class Fault {
    private final String code;
    private final String message;
    private final ResponseClass responseClass;

    Fault(Map<String, Object> value) {
        code = Fields.text(value, "code");
        message = Fields.text(value, "message");
        responseClass = ResponseClass.of(Fields.optionalText(value, "class"));
    }

    /**
     * SUNAT's code.
     *
     * @return {@code 1033}, {@code 0111}...
     */
    public String getCode() {
        return code;
    }

    /**
     * SUNAT's text.
     *
     * @return the message as SUNAT sent it
     */
    public String getMessage() {
        return message;
    }

    /**
     * How the SDK classifies the code.
     *
     * @return {@link ResponseClass#ALREADY_REGISTERED} for 1033, and so on
     */
    public ResponseClass getResponseClass() {
        return responseClass;
    }
}
