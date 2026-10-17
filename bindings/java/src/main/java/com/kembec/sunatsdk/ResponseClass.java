package com.kembec.sunatsdk;

/** The class of a SUNAT response code, as the SDK classifies a CDR or a fault. */
public enum ResponseClass {
    /** A code the SDK does not classify, or none. */
    UNKNOWN(null),
    /** Code 0 without observations. */
    ACCEPTED("accepted"),
    /** 4000 and above, or code 0 with a note of 4000 or above: accepted, with observations. */
    OBSERVED("observed"),
    /** 100 to 1999 but 1032 and 1033: the file was not processed and the document counts as not received. */
    EXCEPTION("exception"),
    /** 1033: SUNAT already has it; fetch its CDR with {@code status}. */
    ALREADY_REGISTERED("already_registered"),
    /** 1032: the number was voided or rejected before. */
    NUMBER_VOIDED_OR_REJECTED("number_voided_or_rejected"),
    /** 2000 to 3999 but 2323 and 2324: rejected; for invoices and notes the number is spent. */
    REJECTED("rejected"),
    /** 2323, 2324: the void was already reported. */
    VOID_ALREADY_REPORTED("void_already_reported");

    private final String wireName;

    ResponseClass(String wireName) {
        this.wireName = wireName;
    }

    /** A name the contract does not have, or none, is {@link #UNKNOWN}: a newer library may add classes. */
    static ResponseClass of(String name) {
        for (ResponseClass value : values()) {
            if (value.wireName != null && value.wireName.equals(name)) {
                return value;
            }
        }
        return UNKNOWN;
    }
}
