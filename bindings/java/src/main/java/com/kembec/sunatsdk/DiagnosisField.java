package com.kembec.sunatsdk;

import java.util.Map;

/** One candidate field for a code, from SUNAT's validation rules of 26.08.2026 (docs/cli.md#diagnosis). */
public final class DiagnosisField {
    /** {@link #getCertainty()} when the node is the rule's own and one field writes it. */
    public static final String SOURCE = "fuente";
    /** {@link #getCertainty()} when the rule computes over several nodes. */
    public static final String INFERRED = "inferido";

    private final String inputPath;
    private final String xpath;
    private final String certainty;

    DiagnosisField(Map<String, Object> member) {
        inputPath = Fields.optionalText(member, "input_path");
        xpath = Fields.optionalText(member, "xpath");
        certainty = Fields.text(member, "certainty");
    }

    /**
     * A field of the input model ({@code customer.document_type}, {@code lines[].unit_price}), {@code sdk} when the SDK
     * writes the node itself.
     *
     * @return null when the input model has no such field
     */
    public String getInputPath() {
        return inputPath;
    }

    /**
     * The node the rule checks.
     *
     * @return null for rules on the file, the signature or the service
     */
    public String getXpath() {
        return xpath;
    }

    /**
     * How sure the rule is about the field.
     *
     * @return {@link #SOURCE} or {@link #INFERRED}
     */
    public String getCertainty() {
        return certainty;
    }
}
