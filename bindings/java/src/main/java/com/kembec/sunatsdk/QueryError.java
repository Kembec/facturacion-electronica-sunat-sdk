package com.kembec.sunatsdk;

import java.util.Map;

/** The Consulta Integrada refused the query (HTTP 200, 400 or 422 with {@code success: false}). */
public final class QueryError {
    private final int httpStatus;
    private final String errorCode;
    private final String message;

    QueryError(Map<String, Object> value) {
        httpStatus = Fields.integer(value, "http_status");
        errorCode = Fields.optionalText(value, "error_code");
        message = Fields.text(value, "message");
    }

    /**
     * The HTTP status of the answer.
     *
     * @return the status
     */
    public int getHttpStatus() {
        return httpStatus;
    }

    /**
     * SUNAT's {@code errorCode}.
     *
     * @return {@code 99} (malformed)..., or null
     */
    public String getErrorCode() {
        return errorCode;
    }

    /**
     * SUNAT's message.
     *
     * @return the message
     */
    public String getMessage() {
        return message;
    }
}
