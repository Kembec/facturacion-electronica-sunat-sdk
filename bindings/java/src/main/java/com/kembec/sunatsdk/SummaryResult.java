package com.kembec.sunatsdk;

import java.util.Map;

/** {@code summary} (sendSummary): the ticket, or SUNAT's fault. */
public final class SummaryResult extends SdkResult {
    private final Outcome outcome;
    private final String ticket;
    private final Fault fault;

    SummaryResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        outcome = Fields.outcome(value, Outcome.TICKET, Outcome.FAULT);
        ticket = outcome == Outcome.TICKET ? Fields.text(value, "ticket") : null;
        fault = outcome == Outcome.FAULT ? new Fault(value) : null;
    }

    /**
     * What SUNAT answered.
     *
     * @return {@link Outcome#TICKET} or {@link Outcome#FAULT}
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
     * SUNAT's fault.
     *
     * @return set with {@link Outcome#FAULT}, null otherwise
     */
    public Fault getFault() {
        return fault;
    }
}
