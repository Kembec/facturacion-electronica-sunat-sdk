package com.kembec.sunatsdk;

import java.util.Map;

/**
 * What SUNAT's message names as {@code (nodo: "<node>" valor: "<value>")}, e.g. {@code cbc:ID/schemeID} = {@code 1}.
 * The form is observed in beta answers; SUNAT does not document it.
 */
public final class DiagnosisDetail {
    private final String node;
    private final String value;

    DiagnosisDetail(Map<String, Object> member) {
        node = Fields.text(member, "node");
        value = Fields.text(member, "value");
    }

    /**
     * The last element and, if any, its attribute, as SUNAT writes it.
     *
     * @return {@code cbc:ID/schemeID}...
     */
    public String getNode() {
        return node;
    }

    /**
     * The value SUNAT read in that node.
     *
     * @return the value as SUNAT quotes it
     */
    public String getValue() {
        return value;
    }
}
