package com.kembec.sunatsdk;

import java.util.Map;

/**
 * {@code send} (sendBill): the CDR, or SUNAT's fault. A send without an answer throws {@link SunatSdkException}: it
 * may have arrived.
 */
public final class SendResult extends SdkResult {
    private final Outcome outcome;
    private final Cdr cdr;
    private final byte[] cdrZip;
    private final Fault fault;

    SendResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        outcome = Fields.outcome(value, Outcome.CDR, Outcome.FAULT);
        if (outcome == Outcome.CDR) {
            Map<String, Object> member = Fields.requiredObject(value, "cdr");
            cdr = new Cdr(member, Json.write(member));
            cdrZip = Fields.optionalBase64(value, "cdr_zip_base64");
            fault = null;
        } else {
            cdr = null;
            cdrZip = null;
            fault = new Fault(value);
        }
    }

    /**
     * What SUNAT answered.
     *
     * @return {@link Outcome#CDR} or {@link Outcome#FAULT}
     */
    public Outcome getOutcome() {
        return outcome;
    }

    /**
     * The CDR read.
     *
     * @return set with {@link Outcome#CDR}, null otherwise
     */
    public Cdr getCdr() {
        return cdr;
    }

    /**
     * The CDR zip as SUNAT sent it, to store.
     *
     * @return a copy, set with {@link Outcome#CDR}; null otherwise
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
}
