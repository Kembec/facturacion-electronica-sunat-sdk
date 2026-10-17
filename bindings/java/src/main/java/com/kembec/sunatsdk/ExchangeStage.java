package com.kembec.sunatsdk;

/** The call of a REST exchange (Consulta Integrada or GRE) an outcome belongs to. */
public enum ExchangeStage {
    /** A SOAP exchange, which has a single call. */
    NONE(null),
    /** The OAuth token: nothing was sent. */
    TOKEN("token"),
    /** The Consulta Integrada query. */
    QUERY("query"),
    /** The GRE send. */
    SEND("send"),
    /** The GRE ticket query. */
    TICKET("ticket");

    private final String wireName;

    ExchangeStage(String wireName) {
        this.wireName = wireName;
    }

    static ExchangeStage of(String name) {
        if (name == null) {
            return NONE;
        }
        for (ExchangeStage value : values()) {
            if (name.equals(value.wireName)) {
                return value;
            }
        }
        throw new Fields.ShapeError("unknown stage `" + name + "`");
    }
}
