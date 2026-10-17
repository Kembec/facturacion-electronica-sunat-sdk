package com.kembec.sunatsdk;

import java.util.List;
import java.util.Map;

/** A CDR (constancia de recepción): the code SUNAT answered and its class. */
public final class Cdr extends SdkResult {
    private final String referenceId;
    private final String responseCode;
    private final String description;
    private final List<String> notes;
    private final ResponseClass responseClass;

    Cdr(Map<String, Object> value, String rawJson) {
        super(rawJson);
        referenceId = Fields.text(value, "reference_id");
        responseCode = Fields.text(value, "response_code");
        description = Fields.text(value, "description");
        notes = Fields.texts(value, "notes");
        responseClass = ResponseClass.of(Fields.optionalText(value, "class"));
    }

    /** The {@code cdr} member of a network response, or null; its raw JSON is that member's. */
    static Cdr member(Map<String, Object> value) {
        Map<String, Object> cdr = Fields.optionalObject(value, "cdr");
        return cdr == null ? null : new Cdr(cdr, Json.write(cdr));
    }

    /**
     * The voucher it answers.
     *
     * @return {@code F001-1}
     */
    public String getReferenceId() {
        return referenceId;
    }

    /**
     * SUNAT's response code.
     *
     * @return {@code 0}, {@code 2047}...
     */
    public String getResponseCode() {
        return responseCode;
    }

    /**
     * SUNAT's description of the code.
     *
     * @return the text as SUNAT sent it
     */
    public String getDescription() {
        return description;
    }

    /**
     * The notes; a code of 4000 or above is an observation.
     *
     * @return each {@code "code - text"}; empty if none
     */
    public List<String> getNotes() {
        return notes;
    }

    /**
     * How the SDK classifies the code and the notes.
     *
     * @return the class; {@link ResponseClass#UNKNOWN} if the SDK does not classify the code
     */
    public ResponseClass getResponseClass() {
        return responseClass;
    }

    /**
     * Code 0 without observations.
     *
     * @return true only for {@link ResponseClass#ACCEPTED}
     */
    public boolean isAccepted() {
        return responseClass == ResponseClass.ACCEPTED;
    }

    /**
     * Accepted, with or without observations: the voucher is valid.
     *
     * @return true for {@link ResponseClass#ACCEPTED} and {@link ResponseClass#OBSERVED}
     */
    public boolean isValidDocument() {
        return responseClass == ResponseClass.ACCEPTED || responseClass == ResponseClass.OBSERVED;
    }
}
