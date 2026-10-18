package com.kembec.sunatsdk.vectors;

import com.kembec.sunatsdk.Diagnosis;
import com.kembec.sunatsdk.DiagnosisDetail;
import com.kembec.sunatsdk.DiagnosisField;
import com.kembec.sunatsdk.Outcome;
import com.kembec.sunatsdk.SdkResult;
import com.kembec.sunatsdk.SendResult;
import com.kembec.sunatsdk.StatusResult;
import com.kembec.sunatsdk.SummaryResult;
import com.kembec.sunatsdk.TicketResult;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

/**
 * The core's answers (tests/data, the ones the PHP and wasm bindings check): the beta's real 2800 to a boleta, a CDR
 * with two coded notes, and answers without {@code diagnosis}. The result types have no public constructor, so they
 * are built the way the client builds them, from the response JSON.
 */
final class DiagnosisVectors {
    private DiagnosisVectors() {
    }

    static void run(Path repo) throws Exception {
        SendResult sent = build(SendResult.class, read(repo, "send-beta-2800.json"));
        Vectors.check(sent.getOutcome() == Outcome.FAULT && "2800".equals(sent.getFault().getCode()), "diagnosis: the 2800 is a fault");
        List<Diagnosis> sentDiagnosis = sent.getDiagnosis();
        Vectors.check(sentDiagnosis.size() == 1 && "2800".equals(sentDiagnosis.get(0).getCode()), "diagnosis: one element, 2800");
        DiagnosisDetail detail = sentDiagnosis.get(0).getDetail();
        Vectors.check(detail != null && "cbc:ID/schemeID".equals(detail.getNode()) && "1".equals(detail.getValue()),
                "diagnosis: detail cbc:ID/schemeID = 1");
        List<DiagnosisField> fields = sentDiagnosis.get(0).getFields();
        Vectors.check(fields.size() == 1 && "customer.document_type".equals(fields.get(0).getInputPath())
                && DiagnosisField.SOURCE.equals(fields.get(0).getCertainty())
                && fields.get(0).getXpath().endsWith("cbc:ID@schemeID"), "diagnosis: customer.document_type, fuente");

        StatusResult status = build(StatusResult.class, read(repo, "status-observed-4287.json"));
        List<Diagnosis> observed = status.getDiagnosis();
        Vectors.check(observed.size() == 2 && "4287".equals(observed.get(0).getCode()) && "4001".equals(observed.get(1).getCode()),
                "diagnosis: 4287 and 4001");
        Vectors.check(observed.get(0).getDetail() == null && "lines[].unit_price".equals(observed.get(0).getFields().get(0).getInputPath())
                && DiagnosisField.INFERRED.equals(observed.get(0).getFields().get(0).getCertainty()),
                "diagnosis: 4287 points at lines[].unit_price, inferido");
        Vectors.check(observed.get(1).getFields().isEmpty(), "diagnosis: 4001 is not in the rules");

        SummaryResult without = build(SummaryResult.class, "{\"outcome\":\"ticket\",\"ticket\":\"1700000000000\"}");
        Vectors.check(without.getDiagnosis().isEmpty(), "diagnosis: absent is an empty list");
        TicketResult fault = build(TicketResult.class, "{\"outcome\":\"fault\",\"code\":\"0109\",\"message\":\"x\",\"class\":null}");
        Vectors.check(fault.getDiagnosis().isEmpty(), "diagnosis: a fault without it is empty");
    }

    private static <T extends SdkResult> T build(Class<T> type, String json) throws Exception {
        Class<?> jsonClass = Class.forName("com.kembec.sunatsdk.Json");
        Method parse = jsonClass.getDeclaredMethod("parse", String.class);
        parse.setAccessible(true);
        Object root = parse.invoke(null, json);
        Constructor<T> constructor = type.getDeclaredConstructor(Map.class, String.class);
        constructor.setAccessible(true);
        return constructor.newInstance(root, json);
    }

    private static String read(Path repo, String name) throws Exception {
        return new String(Files.readAllBytes(repo.resolve("bindings/java/tests/data").resolve(name)), StandardCharsets.UTF_8);
    }
}
