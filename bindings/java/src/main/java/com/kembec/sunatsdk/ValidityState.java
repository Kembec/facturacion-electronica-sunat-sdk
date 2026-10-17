package com.kembec.sunatsdk;

/** The voucher's state in SUNAT's Consulta Integrada. */
public enum ValidityState {
    /** A state code the contract does not name, or none. */
    UNKNOWN(null),
    /** 0: not reported. It does not prove the voucher never arrived. */
    NOT_REPORTED("not_reported"),
    /** 1: accepted. */
    ACCEPTED("accepted"),
    /** 2: voided. */
    VOIDED("voided"),
    /** 3: printed, authorized. */
    PRINTED_AUTHORIZED("printed_authorized"),
    /** 4: printed, not authorized. */
    PRINTED_NOT_AUTHORIZED("printed_not_authorized");

    private final String wireName;

    ValidityState(String wireName) {
        this.wireName = wireName;
    }

    static ValidityState of(String name) {
        for (ValidityState value : values()) {
            if (value.wireName != null && value.wireName.equals(name)) {
                return value;
            }
        }
        return UNKNOWN;
    }
}
