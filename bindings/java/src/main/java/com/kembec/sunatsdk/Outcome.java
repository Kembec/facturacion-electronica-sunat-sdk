package com.kembec.sunatsdk;

/** The {@code outcome} of a network command (docs/cli.md). Each result says which ones it can carry. */
public enum Outcome {
    /** {@code send}: SUNAT answered with a CDR. */
    CDR("cdr"),
    /** A SOAP fault: SUNAT refused the call with a code ({@code 1033}, {@code 0111}...). */
    FAULT("fault"),
    /** {@code summary} and {@code gre-send}: SUNAT received the file and gave a ticket. */
    TICKET("ticket"),
    /** {@code ticket} and {@code gre-ticket}: the state of a ticket. */
    TICKET_STATUS("ticket_status"),
    /** {@code status}: getStatusCdr's answer. */
    STATUS("status"),
    /** No answer, or an HTTP status that says nothing about the document: ask again. */
    NO_VERDICT("no_verdict"),
    /** {@code validity}: the voucher's state in the Consulta Integrada. */
    VALIDITY("validity"),
    /** {@code validity}: SUNAT refused the query (not the voucher). */
    QUERY_ERROR("query_error"),
    /** {@code gre-send} and {@code gre-ticket}: an error body of the GRE REST API. */
    API_ERROR("api_error");

    private final String wireName;

    Outcome(String wireName) {
        this.wireName = wireName;
    }

    /**
     * The name in the contract's JSON.
     *
     * @return {@code no_verdict} for {@link #NO_VERDICT}, and so on
     */
    public String wireName() {
        return wireName;
    }
}
