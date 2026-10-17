package com.kembec.sunatsdk;

import java.util.Map;

/**
 * No answer, or an HTTP status that says nothing about the document. It is not a rejection: ask again (docs/cli.md
 * says when to resend the same signed XML with the same number).
 */
public final class NoVerdict {
    private final NoVerdictReason reason;
    private final Integer httpStatus;
    private final String message;
    private final ExchangeStage stage;

    NoVerdict(Map<String, Object> value) {
        reason = NoVerdictReason.of(Fields.text(value, "reason"));
        httpStatus = Fields.optionalInt(value, "http_status");
        message = Fields.text(value, "message");
        stage = ExchangeStage.of(Fields.optionalText(value, "stage"));
    }

    /**
     * Why there is no verdict.
     *
     * @return the wait ran out, or SUNAT answered an HTTP status
     */
    public NoVerdictReason getReason() {
        return reason;
    }

    /**
     * The HTTP status.
     *
     * @return 401..., or null after a timeout
     */
    public Integer getHttpStatus() {
        return httpStatus;
    }

    /**
     * What happened.
     *
     * @return a message in English
     */
    public String getMessage() {
        return message;
    }

    /**
     * For the REST exchanges, the call without an answer.
     *
     * @return {@link ExchangeStage#TOKEN} means nothing was sent; {@link ExchangeStage#NONE} for SOAP
     */
    public ExchangeStage getStage() {
        return stage;
    }
}
