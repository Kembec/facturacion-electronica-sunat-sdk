package com.kembec.sunatsdk;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;

/** A code SUNAT answered, with the input fields that may have caused it (the {@code diagnosis} of docs/cli.md). */
public final class Diagnosis {
    private final String code;
    private final DiagnosisDetail detail;
    private final List<DiagnosisField> fields;

    Diagnosis(Map<String, Object> value) {
        code = Fields.text(value, "code");
        Map<String, Object> member = Fields.optionalObject(value, "detail");
        detail = member == null ? null : new DiagnosisDetail(member);
        List<DiagnosisField> candidates = new ArrayList<DiagnosisField>();
        for (Map<String, Object> field : Fields.objects(value, "fields")) {
            candidates.add(new DiagnosisField(field));
        }
        fields = Collections.unmodifiableList(candidates);
    }

    /** The {@code diagnosis} of an outcome; absent or null is empty. */
    static List<Diagnosis> listFrom(Map<String, Object> value) {
        List<Diagnosis> list = new ArrayList<Diagnosis>();
        for (Map<String, Object> item : Fields.objects(value, "diagnosis")) {
            list.add(new Diagnosis(item));
        }
        return Collections.unmodifiableList(list);
    }

    /**
     * The code SUNAT answered.
     *
     * @return {@code 2800}, {@code 4287}...
     */
    public String getCode() {
        return code;
    }

    /**
     * What SUNAT's message names (node and value).
     *
     * @return null when it names none
     */
    public DiagnosisDetail getDetail() {
        return detail;
    }

    /**
     * The candidate fields; with a detail, only those on its node, unless none is.
     *
     * @return an unmodifiable list; empty for a code the rules do not list, and for summaries, void communications and
     *     tickets
     */
    public List<DiagnosisField> getFields() {
        return fields;
    }
}
