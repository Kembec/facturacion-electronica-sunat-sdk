package com.kembec.sunatsdk;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;

/** An error body of the GRE REST API (manual GRE-M, §1.3 and annex I). */
public final class GreApiError {
    private final ExchangeStage stage;
    private final int httpStatus;
    private final String generalError;
    private final String code;
    private final String message;
    private final List<GreApiErrorItem> errors;

    GreApiError(Map<String, Object> value) {
        stage = ExchangeStage.of(Fields.optionalText(value, "stage"));
        httpStatus = Fields.integer(value, "http_status");
        generalError = Fields.optionalText(value, "general_error");
        code = Fields.optionalText(value, "code");
        message = Fields.text(value, "message");
        List<GreApiErrorItem> items = new ArrayList<GreApiErrorItem>();
        for (Map<String, Object> item : Fields.objects(value, "errors")) {
            items.add(new GreApiErrorItem(Fields.optionalText(item, "code"), Fields.text(item, "description")));
        }
        errors = Collections.unmodifiableList(items);
    }

    /**
     * The call SUNAT answered.
     *
     * @return {@link ExchangeStage#TOKEN}, {@link ExchangeStage#SEND} or {@link ExchangeStage#TICKET}
     */
    public ExchangeStage getStage() {
        return stage;
    }

    /**
     * The HTTP status.
     *
     * @return the status
     */
    public int getHttpStatus() {
        return httpStatus;
    }

    /**
     * Annex I's name of the status.
     *
     * @return {@code unauthorized}, {@code unprocessable_entity}..., or null
     */
    public String getGeneralError() {
        return generalError;
    }

    /**
     * SUNAT's {@code cod}.
     *
     * @return the code, or null
     */
    public String getCode() {
        return code;
    }

    /**
     * SUNAT's {@code msg}.
     *
     * @return the message
     */
    public String getMessage() {
        return message;
    }

    /**
     * With 422, what SUNAT rejected, in its words.
     *
     * @return the errors; empty if none
     */
    public List<GreApiErrorItem> getErrors() {
        return errors;
    }
}
