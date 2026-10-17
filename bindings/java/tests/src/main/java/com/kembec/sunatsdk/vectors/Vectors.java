package com.kembec.sunatsdk.vectors;

import com.kembec.sunatsdk.ApiCredentials;
import com.kembec.sunatsdk.BuildResult;
import com.kembec.sunatsdk.Cdr;
import com.kembec.sunatsdk.ExchangeStage;
import com.kembec.sunatsdk.GreTicketResult;
import com.kembec.sunatsdk.NoVerdictReason;
import com.kembec.sunatsdk.Outcome;
import com.kembec.sunatsdk.ResponseClass;
import com.kembec.sunatsdk.SignResult;
import com.kembec.sunatsdk.SigningKey;
import com.kembec.sunatsdk.SolCredentials;
import com.kembec.sunatsdk.StatusResult;
import com.kembec.sunatsdk.SunatSdkClient;
import com.kembec.sunatsdk.SunatSdkException;
import com.kembec.sunatsdk.ValidityResult;
import com.kembec.sunatsdk.ZipResult;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.math.BigDecimal;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Duration;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.Callable;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import javax.crypto.spec.SecretKeySpec;
import javax.xml.crypto.Data;
import javax.xml.crypto.OctetStreamData;
import javax.xml.crypto.dsig.CanonicalizationMethod;
import javax.xml.crypto.dsig.Transform;
import javax.xml.crypto.dsig.XMLSignatureFactory;
import javax.xml.crypto.dsig.dom.DOMValidateContext;
import javax.xml.crypto.dsig.spec.TransformParameterSpec;
import javax.xml.parsers.DocumentBuilderFactory;

/**
 * The battery of the package as a consumer resolves it from its jar: the 38 fixtures against their vectors (inclusive
 * C14N, exact DigestValue and SignatureValue), the native version against the jar's, zip and CDR, the errors,
 * no_verdict without an answer and concurrent calls. Exit code 0 only if every check passed.
 */
public final class Vectors {
    private static final int EXPECTED_FIXTURES = 38;
    private static final AtomicInteger CHECKS = new AtomicInteger();
    private static final AtomicInteger FAILURES = new AtomicInteger();

    private Vectors() {
    }

    /** Something to run that may throw anything, reported as a failed check. */
    private interface Step {
        void run() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            System.err.println("usage: Vectors <repository>");
            System.exit(2);
        }
        final Path repo = Paths.get(args[0]).toAbsolutePath().normalize();
        final SigningKey key = new SigningKey(
                repo.resolve("tests/vectors/test-cert/test-cert.pem"), repo.resolve("tests/vectors/test-cert/test-key.pem"));
        final SunatSdkClient client = new SunatSdkClient(Duration.ofSeconds(30));

        run("version", Vectors::version);
        List<Path> fixtures = fixtures(repo);
        check(fixtures.size() == EXPECTED_FIXTURES, "fixtures: " + fixtures.size() + ", expected " + EXPECTED_FIXTURES);
        for (final Path fixture : fixtures) {
            run(name(fixture), () -> vector(client, key, repo, fixture));
        }
        run("zip", () -> zip(client, key, repo));
        run("cdr", () -> readCdr(client, repo));
        run("errors", () -> errors(client, key, repo));
        run("no_verdict", Vectors::noVerdicts);
        run("threads", () -> threads(client, key, repo, fixtures));

        String runtime = System.getProperty("sun.arch.data.model") + "-bit Java " + System.getProperty("java.version")
                + " (" + System.getProperty("java.vendor") + ", " + System.getProperty("os.name") + " " + System.getProperty("os.arch") + ")";
        if (FAILURES.get() > 0) {
            System.err.println("Java: " + FAILURES.get() + " of " + CHECKS.get() + " checks failed, " + runtime);
            System.exit(1);
        }
        System.out.println("ok Java: " + CHECKS.get() + " checks, " + fixtures.size() + " vectors, " + runtime
                + ", native " + SunatSdkClient.nativeVersion());
    }

    private static void version() {
        String jar = SunatSdkClient.class.getPackage().getImplementationVersion();
        check(SunatSdkClient.nativeVersion().equals(jar), "native " + SunatSdkClient.nativeVersion() + ", jar " + jar);
    }

    private static void vector(SunatSdkClient client, SigningKey key, Path repo, Path fixture) throws Exception {
        String name = name(fixture);
        String document = read(fixture);
        BuildResult built = client.build(document);
        check(canonical(vectorXml(repo, name, "unsigned")).equals(canonical(built.getXml())), name + ": unsigned XML differs from its vector under C14N");
        check(built.getFileName().equals(expectedFileName(document)), name + ": file name " + built.getFileName());

        SignResult signed = client.sign(built.getXml(), key);
        String expected = vectorXml(repo, name, "signed");
        check(canonical(expected).equals(canonical(signed.getXml())), name + ": signed XML differs from its vector under C14N");
        check(expected.contains("<ds:DigestValue>" + signed.getDigestValue() + "</ds:DigestValue>"), name + ": DigestValue " + signed.getDigestValue());
        check(expected.contains("<ds:SignatureValue>" + signed.getSignatureValue() + "</ds:SignatureValue>"), name + ": SignatureValue differs");
        check(signed.getXml().contains("<ds:DigestValue>" + signed.getDigestValue() + "</ds:DigestValue>"), name + ": DigestValue not in the signed XML");
    }

    private static void zip(SunatSdkClient client, SigningKey key, Path repo) throws IOException {
        String signed = signFixture(client, key, repo, "01-invoice-taxed-cash");
        ZipResult zip = client.zip("20123456789-01-F001-1", signed);
        byte[] content = zip.getContent();
        check(zip.getFileName().equals("20123456789-01-F001-1.zip"), "zip: file name " + zip.getFileName());
        check(content.length > 4 && content[0] == 'P' && content[1] == 'K', "zip: not a zip");
    }

    private static void readCdr(SunatSdkClient client, Path repo) throws IOException {
        byte[] bytes = Files.readAllBytes(repo.resolve("crates/sunat-transport/tests/data/R-20150147718-01-FT01-982.zip"));
        Cdr cdr = client.readCdr(bytes);
        check(cdr.getReferenceId().equals("FT01-982"), "cdr: reference " + cdr.getReferenceId());
        check(cdr.getResponseCode().equals("2047"), "cdr: code " + cdr.getResponseCode());
        check(cdr.getResponseClass() == ResponseClass.REJECTED && !cdr.isValidDocument(), "cdr: class " + cdr.getResponseClass());
    }

    private static void errors(final SunatSdkClient client, SigningKey key, Path repo) throws IOException {
        SunatSdkException build = expect(SunatSdkException.class, () -> client.build("{}"), "build of {} is the SDK's error");
        check(build == null || "build".equals(build.getCommand()), "build of {}: command " + (build == null ? null : build.getCommand()));
        IllegalArgumentException json = expect(IllegalArgumentException.class, () -> client.build("{not json"), "invalid JSON");
        check(json == null || json.getMessage().contains("documentJson"), "invalid JSON names the parameter");
        expect(IllegalArgumentException.class, () -> new SunatSdkClient(Duration.ofMillis(1500)), "timeout of 1.5 s");
        expect(NullPointerException.class, () -> client.build(null), "null document");

        final SolCredentials sol = new SolCredentials("20123456789MODDATOS", "moddatos-secret");
        check(!sol.toString().contains("moddatos-secret"), "SolCredentials.toString shows the password");
        final String signed = signFixture(client, key, repo, "01-invoice-taxed-cash");
        SunatSdkException http = expect(SunatSdkException.class,
                () -> client.send("20123456789-01-F001-1", signed, sol, "http://127.0.0.1/billService"), "send over http://");
        check(http == null || ("send".equals(http.getCommand()) && http.getMessage().contains("endpoint must use https://")
                && !http.getMessage().contains("moddatos-secret")), "send over http://: " + (http == null ? null : http.getMessage()));
    }

    /** A listener that accepts and never answers the TLS handshake: every query waits out its 1 s. */
    private static void noVerdicts() throws Exception {
        final ServerSocket listener = new ServerSocket(0, 50, InetAddress.getLoopbackAddress());
        final List<Socket> held = new ArrayList<Socket>();
        Thread accepting = new Thread(() -> {
            try {
                while (true) {
                    Socket socket = listener.accept();
                    synchronized (held) {
                        held.add(socket);
                    }
                }
            } catch (IOException closed) {
                // the listener was closed: the test is over
            }
        });
        accepting.setDaemon(true);
        accepting.start();
        try {
            String silent = "https://127.0.0.1:" + listener.getLocalPort();
            SunatSdkClient client = new SunatSdkClient(Duration.ofSeconds(1));
            SolCredentials sol = new SolCredentials("20123456789MODDATOS", "moddatos");
            ApiCredentials api = new ApiCredentials("client-id", "client-secret");

            StatusResult status = client.status("20123456789", "01", "F001", 1, sol, silent + "/billConsultService");
            check(status.getOutcome() == Outcome.NO_VERDICT && status.getNoVerdict().getReason() == NoVerdictReason.TIMEOUT
                    && status.getNoVerdict().getHttpStatus() == null, "status without an answer: " + status.getRawJson());

            ValidityResult validity = client.validity("20123456789", "01", "F001", 1, LocalDate.of(2026, 10, 1),
                    new BigDecimal("118.00"), api, null, silent + "/token", silent + "/query");
            check(validity.getOutcome() == Outcome.NO_VERDICT && validity.getNoVerdict().getStage() == ExchangeStage.TOKEN,
                    "validity without a token: " + validity.getRawJson());

            GreTicketResult gre = client.greTicket("1700000000000", api, sol, silent + "/token", silent + "/v1");
            check(gre.getOutcome() == Outcome.NO_VERDICT && gre.getNoVerdict().getStage() == ExchangeStage.TOKEN
                    && gre.getTicket().equals("1700000000000"), "gre-ticket without a token: " + gre.getRawJson());
        } finally {
            listener.close();
            synchronized (held) {
                for (Socket socket : held) {
                    socket.close();
                }
            }
        }
    }

    /** Eight threads build and sign different fixtures at once, five rounds; each must still match its vector. */
    private static void threads(final SunatSdkClient client, final SigningKey key, final Path repo, List<Path> fixtures) throws Exception {
        List<Path> chosen = fixtures.subList(0, Math.min(8, fixtures.size()));
        ExecutorService pool = Executors.newFixedThreadPool(chosen.size());
        try {
            List<Future<String>> digests = new ArrayList<Future<String>>();
            for (final Path fixture : chosen) {
                digests.add(pool.submit(new Callable<String>() {
                    @Override
                    public String call() throws Exception {
                        String digest = null;
                        for (int round = 0; round < 5; round++) {
                            digest = client.sign(client.build(read(fixture)).getXml(), key).getDigestValue();
                        }
                        return digest;
                    }
                }));
            }
            for (int i = 0; i < chosen.size(); i++) {
                String name = name(chosen.get(i));
                check(vectorXml(repo, name, "signed").contains("<ds:DigestValue>" + digests.get(i).get() + "</ds:DigestValue>"), "threads: " + name);
            }
        } finally {
            pool.shutdownNow();
        }
    }

    private static String signFixture(SunatSdkClient client, SigningKey key, Path repo, String name) throws IOException {
        BuildResult built = client.build(read(repo.resolve("fixtures/" + name + ".json")));
        return client.sign(built.getXml(), key).getXml();
    }

    /** {@code ruc-type-series-number}, or {@code ruc-RC|RA-YYYYMMDD-number}, from the fixture's own fields. */
    private static String expectedFileName(String document) {
        String ruc = member(member(document, "issuer", "\\{[^}]*"), "ruc", null);
        String number = member(document, "number", null);
        String kind = member(document, "kind", null);
        if ("summary".equals(kind)) {
            return ruc + "-RC-" + member(document, "generation_date", null).replace("-", "") + "-" + number;
        }
        if ("voided".equals(kind)) {
            return ruc + "-RA-" + member(document, "communication_date", null).replace("-", "") + "-" + number;
        }
        return ruc + "-" + member(document, "document_type", null) + "-" + member(document, "series", null) + "-" + number;
    }

    /**
     * The first string member {@code name} of the fixture, or the raw text of an object when {@code shape} is given.
     * Enough for the top-level keys of the fixtures, which are not escaped; the test does not use the package's parser
     * to check the package.
     */
    private static String member(String json, String name, String shape) {
        String value = shape == null ? "\"([^\"]*)\"" : "(" + shape + ")";
        Matcher m = Pattern.compile("\"" + Pattern.quote(name) + "\"\\s*:\\s*" + value).matcher(json);
        if (!m.find()) {
            throw new IllegalStateException("no `" + name + "` in the fixture");
        }
        return m.group(1);
    }

    /** Inclusive C14N 1.0 without comments, the form the signature is computed over, from the JDK's XML-DSig. */
    private static String canonical(String xml) throws Exception {
        Transform c14n = XMLSignatureFactory.getInstance("DOM")
                .newTransform(CanonicalizationMethod.INCLUSIVE, (TransformParameterSpec) null);
        DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
        factory.setNamespaceAware(true);
        DOMValidateContext context = new DOMValidateContext(new SecretKeySpec(new byte[] {1}, "HmacSHA1"), factory.newDocumentBuilder().newDocument());
        // transform(data, context) and not the OutputStream overload, which needs the transform marshalled in a signature.
        Data result = c14n.transform(new OctetStreamData(new ByteArrayInputStream(xml.getBytes(StandardCharsets.UTF_8))), context);
        InputStream in = ((OctetStreamData) result).getOctetStream();
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] chunk = new byte[8192];
        for (int n = in.read(chunk); n >= 0; n = in.read(chunk)) {
            out.write(chunk, 0, n);
        }
        return new String(out.toByteArray(), StandardCharsets.UTF_8);
    }

    private static List<Path> fixtures(Path repo) {
        File[] files = repo.resolve("fixtures").toFile().listFiles((dir, file) -> file.endsWith(".json"));
        List<Path> paths = new ArrayList<Path>();
        if (files != null) {
            Arrays.sort(files);
            for (File file : files) {
                paths.add(file.toPath());
            }
        }
        return paths;
    }

    private static String name(Path fixture) {
        String file = fixture.getFileName().toString();
        return file.substring(0, file.length() - ".json".length());
    }

    private static String vectorXml(Path repo, String name, String kind) throws IOException {
        return read(repo.resolve("tests/vectors/" + name + "." + kind + ".xml"));
    }

    private static String read(Path path) throws IOException {
        return new String(Files.readAllBytes(path), StandardCharsets.UTF_8);
    }

    /** Runs {@code action}, which must throw exactly {@code type}; returns it, or null after a failed check. */
    private static <T extends Throwable> T expect(Class<T> type, Step action, String what) {
        try {
            action.run();
            check(false, what + ": no exception");
        } catch (Throwable e) {
            if (e.getClass() == type) {
                CHECKS.incrementAndGet();
                return type.cast(e);
            }
            check(false, what + ": " + e.getClass().getName() + " " + e.getMessage());
        }
        return null;
    }

    private static void run(String what, Step test) {
        try {
            test.run();
        } catch (Throwable e) {
            check(false, what + ": " + e.getClass().getName() + " " + e.getMessage());
        }
    }

    private static void check(boolean ok, String what) {
        CHECKS.incrementAndGet();
        if (!ok) {
            FAILURES.incrementAndGet();
            System.err.println("FAIL " + what);
        }
    }
}
