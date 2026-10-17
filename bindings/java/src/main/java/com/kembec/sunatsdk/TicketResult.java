package com.kembec.sunatsdk;

import java.util.Map;

/** {@code ticket} (getStatus): the state of a summary's or a void's ticket. */
public final class TicketResult extends SdkResult {
    private final Outcome outcome;
    private final String ticket;
    private final String statusCode;
    private final TicketState state;
    private final Cdr cdr;
    private final byte[] cdrZip;
    private final Fault fault;
    private final NoVerdict noVerdict;

    TicketResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        outcome = Fields.outcome(value, Outcome.TICKET_STATUS, Outcome.FAULT, Outcome.NO_VERDICT);
        ticket = Fields.optionalText(value, "ticket");
        boolean status = outcome == Outcome.TICKET_STATUS;
        statusCode = status ? Fields.text(value, "status_code") : null;
        state = status ? TicketState.of(Fields.optionalText(value, "state")) : TicketState.UNKNOWN;
        cdr = status ? Cdr.member(value) : null;
        cdrZip = status ? Fields.optionalBase64(value, "cdr_zip_base64") : null;
        fault = outcome == Outcome.FAULT ? new Fault(value) : null;
        noVerdict = outcome == Outcome.NO_VERDICT ? new NoVerdict(value) : null;
    }

    /**
     * What SUNAT answered.
     *
     * @return {@link Outcome#TICKET_STATUS}, {@link Outcome#FAULT} or {@link Outcome#NO_VERDICT}
     */
    public Outcome getOutcome() {
        return outcome;
    }

    /**
     * The ticket asked about.
     *
     * @return set with {@link Outcome#NO_VERDICT}; null when SUNAT's answer does not repeat it
     */
    public String getTicket() {
        return ticket;
    }

    /**
     * SUNAT's status code.
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
     * No answer: ask again; the ticket proves the file arrived.
     *
     * @return set with {@link Outcome#NO_VERDICT}, null otherwise
     */
    public NoVerdict getNoVerdict() {
        return noVerdict;
    }
}
