package com.kembec.sunatsdk;

import java.util.Map;

/** {@code status} (getStatusCdr): one voucher's status, with its CDR when SUNAT attaches it. */
public final class StatusResult extends SdkResult {
    private final Outcome outcome;
    private final String statusCode;
    private final String statusMessage;
    private final Cdr cdr;
    private final byte[] cdrZip;
    private final Fault fault;
    private final NoVerdict noVerdict;

    StatusResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        outcome = Fields.outcome(value, Outcome.STATUS, Outcome.FAULT, Outcome.NO_VERDICT);
        boolean status = outcome == Outcome.STATUS;
        statusCode = status ? Fields.text(value, "status_code") : null;
        statusMessage = status ? Fields.optionalText(value, "status_message") : null;
        cdr = status ? Cdr.member(value) : null;
        cdrZip = status ? Fields.optionalBase64(value, "cdr_zip_base64") : null;
        fault = outcome == Outcome.FAULT ? new Fault(value) : null;
        noVerdict = outcome == Outcome.NO_VERDICT ? new NoVerdict(value) : null;
    }

    /**
     * What SUNAT answered.
     *
     * @return {@link Outcome#STATUS}, {@link Outcome#FAULT} or {@link Outcome#NO_VERDICT}
     */
    public Outcome getOutcome() {
        return outcome;
    }

    /**
     * getStatusCdr's {@code statusCode}.
     *
     * @return {@code 0001} (the voucher exists and has a CDR)...; null without {@link Outcome#STATUS}
     */
    public String getStatusCode() {
        return statusCode;
    }

    /**
     * getStatusCdr's {@code statusMessage}.
     *
     * @return the message, or null
     */
    public String getStatusMessage() {
        return statusMessage;
    }

    /**
     * The CDR, when SUNAT attaches it.
     *
     * @return the CDR, or null
     */
    public Cdr getCdr() {
        return cdr;
    }

    /**
     * The CDR zip, when SUNAT attaches it.
     *
     * @return a copy, or null
     */
    public byte[] getCdrZip() {
        return Fields.copy(cdrZip);
    }

    /**
     * SUNAT's fault.
     *
     * @return set with {@link Outcome#FAULT}, null otherwise
     */
    public Fault getFault() {
        return fault;
    }

    /**
     * No answer: ask again.
     *
     * @return set with {@link Outcome#NO_VERDICT}, null otherwise
     */
    public NoVerdict getNoVerdict() {
        return noVerdict;
    }
}
