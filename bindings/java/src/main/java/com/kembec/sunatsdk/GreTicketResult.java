package com.kembec.sunatsdk;

import java.util.Map;

/** {@code gre-ticket}: the state of a despatch advice's ticket. */
public final class GreTicketResult extends SdkResult {
    private final Outcome outcome;
    private final String ticket;
    private final String statusCode;
    private final TicketState state;
    private final boolean cdrGenerated;
    private final GreTicketError error;
    private final Cdr cdr;
    private final byte[] cdrZip;
    private final GreApiError apiError;
    private final NoVerdict noVerdict;

    GreTicketResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        outcome = Fields.outcome(value, Outcome.TICKET_STATUS, Outcome.API_ERROR, Outcome.NO_VERDICT);
        ticket = Fields.text(value, "ticket");
        boolean status = outcome == Outcome.TICKET_STATUS;
        statusCode = status ? Fields.text(value, "status_code") : null;
        state = status ? TicketState.of(Fields.optionalText(value, "state")) : TicketState.UNKNOWN;
        cdrGenerated = status && Fields.bool(value, "cdr_generated");
        Map<String, Object> errorMember = status ? Fields.optionalObject(value, "error") : null;
        error = errorMember == null ? null : new GreTicketError(errorMember);
        cdr = status ? Cdr.member(value) : null;
        cdrZip = status ? Fields.optionalBase64(value, "cdr_zip_base64") : null;
        apiError = outcome == Outcome.API_ERROR ? new GreApiError(value) : null;
        noVerdict = outcome == Outcome.NO_VERDICT ? new NoVerdict(value) : null;
    }

    /**
     * What SUNAT answered.
     *
     * @return {@link Outcome#TICKET_STATUS}, {@link Outcome#API_ERROR} or {@link Outcome#NO_VERDICT}
     */
    public Outcome getOutcome() {
        return outcome;
    }

    /**
     * The ticket asked about.
     *
     * @return the ticket
     */
    public String getTicket() {
        return ticket;
    }

    /**
     * SUNAT's {@code codRespuesta}.
     *
     * @return {@code 0}, {@code 98}, {@code 99}; null without {@link Outcome#TICKET_STATUS}
     */
    public String getStatusCode() {
        return statusCode;
    }

    /**
     * The ticket's state. {@link TicketState#IN_PROCESS}: ask again with the same ticket, never send again.
     *
     * @return the state; {@link TicketState#UNKNOWN} without {@link Outcome#TICKET_STATUS}
     */
    public TicketState getState() {
        return state;
    }

    /**
     * SUNAT's {@code indCdrGenerado} is 1.
     *
     * @return whether SUNAT generated the CDR
     */
    public boolean isCdrGenerated() {
        return cdrGenerated;
    }

    /**
     * SUNAT's {@code error} ({@code numError}, {@code desError}), with 99.
     *
     * @return the error, or null
     */
    public GreTicketError getError() {
        return error;
    }

    /**
     * The CDR, when generated.
     *
     * @return the CDR, or null
     */
    public Cdr getCdr() {
        return cdr;
    }

    /**
     * The CDR zip, when generated.
     *
     * @return a copy, or null
     */
    public byte[] getCdrZip() {
        return Fields.copy(cdrZip);
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
     * No answer: ask again with the same ticket.
     *
     * @return set with {@link Outcome#NO_VERDICT}, null otherwise
     */
    public NoVerdict getNoVerdict() {
        return noVerdict;
    }
}
