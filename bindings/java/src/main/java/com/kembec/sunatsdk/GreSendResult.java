package com.kembec.sunatsdk;

import java.util.Map;

/** {@code gre-send}: the ticket of a despatch advice, an error of the GRE API, or no answer. */
public final class GreSendResult extends SdkResult {
    private final Outcome outcome;
    private final String ticket;
    private final String receivedAt;
    private final String zipName;
    private final String hashZip;
    private final GreApiError apiError;
    private final NoVerdict noVerdict;

    GreSendResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        outcome = Fields.outcome(value, Outcome.TICKET, Outcome.API_ERROR, Outcome.NO_VERDICT);
        boolean ticketed = outcome == Outcome.TICKET;
        ticket = ticketed ? Fields.text(value, "ticket") : null;
        receivedAt = ticketed ? Fields.optionalText(value, "received_at") : null;
        zipName = ticketed ? Fields.optionalText(value, "zip_name") : null;
        hashZip = ticketed ? Fields.optionalText(value, "hash_zip") : null;
        apiError = outcome == Outcome.API_ERROR ? new GreApiError(value) : null;
        noVerdict = outcome == Outcome.NO_VERDICT ? new NoVerdict(value) : null;
    }

    /**
     * What SUNAT answered.
     *
     * @return {@link Outcome#TICKET}, {@link Outcome#API_ERROR} or {@link Outcome#NO_VERDICT} (no token: nothing
     *     was sent)
     */
    public Outcome getOutcome() {
        return outcome;
    }

    /**
     * The ticket. Store it before anything else: it is the only handle on the outcome.
     *
     * @return set with {@link Outcome#TICKET}, null otherwise
     */
    public String getTicket() {
        return ticket;
    }

    /**
     * SUNAT's {@code fecRecepcion}, as it sent it.
     *
     * @return the text, or null
     */
    public String getReceivedAt() {
        return receivedAt;
    }

    /**
     * The name of the zip sent.
     *
     * @return the name, or null
     */
    public String getZipName() {
        return zipName;
    }

    /**
     * SHA-256 of the zip.
     *
     * @return lowercase hex, or null
     */
    public String getHashZip() {
        return hashZip;
    }

    /**
     * An error body of the GRE API.
     *
     * @return set with {@link Outcome#API_ERROR}, null otherwise
     */
    public GreApiError getApiError() {
        return apiError;
    }

    /**
     * No answer.
     *
     * @return set with {@link Outcome#NO_VERDICT}, null otherwise
     */
    public NoVerdict getNoVerdict() {
        return noVerdict;
    }
}
