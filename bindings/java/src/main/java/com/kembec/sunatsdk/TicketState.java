package com.kembec.sunatsdk;

/** The state of a ticket. */
public enum TicketState {
    /** A status code the contract does not name, or no state. */
    UNKNOWN(null),
    /** 0: processed. */
    PROCESSED("processed"),
    /** 98: in process; ask again with the same ticket, never send again. */
    IN_PROCESS("in_process"),
    /** 99: processed with errors. */
    PROCESSED_WITH_ERRORS("processed_with_errors");

    private final String wireName;

    TicketState(String wireName) {
        this.wireName = wireName;
    }

    static TicketState of(String name) {
        for (TicketState value : values()) {
            if (value.wireName != null && value.wireName.equals(name)) {
                return value;
            }
        }
        return UNKNOWN;
    }
}
