package com.kembec.sunatsdk;

import java.util.Collections;
import java.util.List;
import java.util.Map;

/** {@code validity} (Consulta Integrada): the voucher's state, a refused query, or no verdict. */
public final class ValidityResult extends SdkResult {
    private final Outcome outcome;
    private final String stateCode;
    private final ValidityState state;
    private final String taxpayerStateCode;
    private final String domicileConditionCode;
    private final List<String> observations;
    private final String message;
    private final QueryError queryError;
    private final NoVerdict noVerdict;

    ValidityResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        outcome = Fields.outcome(value, Outcome.VALIDITY, Outcome.QUERY_ERROR, Outcome.NO_VERDICT);
        boolean validity = outcome == Outcome.VALIDITY;
        stateCode = validity ? Fields.optionalText(value, "state_code") : null;
        state = validity ? ValidityState.of(Fields.optionalText(value, "state")) : ValidityState.UNKNOWN;
        taxpayerStateCode = validity ? Fields.optionalText(value, "taxpayer_state_code") : null;
        domicileConditionCode = validity ? Fields.optionalText(value, "domicile_condition_code") : null;
        observations = validity ? Fields.texts(value, "observations") : Collections.<String>emptyList();
        message = validity ? Fields.optionalText(value, "message") : null;
        queryError = outcome == Outcome.QUERY_ERROR ? new QueryError(value) : null;
        noVerdict = outcome == Outcome.NO_VERDICT ? new NoVerdict(value) : null;
    }

    /**
     * What SUNAT answered.
     *
     * @return {@link Outcome#VALIDITY}, {@link Outcome#QUERY_ERROR} or {@link Outcome#NO_VERDICT}
     */
    public Outcome getOutcome() {
        return outcome;
    }

    /**
     * SUNAT's {@code estadoCp}.
     *
     * @return {@code 0} to {@code 4}, or null
     */
    public String getStateCode() {
        return stateCode;
    }

    /**
     * The voucher's state. No state is a reason to issue the operation with another number.
     *
     * @return the state; {@link ValidityState#UNKNOWN} without {@link Outcome#VALIDITY}
     */
    public ValidityState getState() {
        return state;
    }

    /**
     * SUNAT's {@code estadoRuc} of the issuer.
     *
     * @return the code, or null
     */
    public String getTaxpayerStateCode() {
        return taxpayerStateCode;
    }

    /**
     * SUNAT's {@code condDomiRuc} of the issuer.
     *
     * @return the code, or null
     */
    public String getDomicileConditionCode() {
        return domicileConditionCode;
    }

    /**
     * Where the data sent differs from SUNAT's (an accepted voucher with another issue date...).
     *
     * @return the observations; empty if none
     */
    public List<String> getObservations() {
        return observations;
    }

    /**
     * SUNAT's message.
     *
     * @return the message, or null
     */
    public String getMessage() {
        return message;
    }

    /**
     * SUNAT refused the query.
     *
     * @return set with {@link Outcome#QUERY_ERROR}, null otherwise
     */
    public QueryError getQueryError() {
        return queryError;
    }

    /**
     * No answer: ask again with the same data.
     *
     * @return set with {@link Outcome#NO_VERDICT}, null otherwise
     */
    public NoVerdict getNoVerdict() {
        return noVerdict;
    }
}
