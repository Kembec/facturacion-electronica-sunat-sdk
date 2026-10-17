package com.kembec.sunatsdk;

import java.math.BigDecimal;
import java.time.Duration;
import java.time.LocalDate;
import java.util.Base64;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Objects;

/**
 * The commands of the SDK's contract (docs/cli.md) in process, through its C ABI (docs/c-abi.md): the same validation,
 * XML, signature, network calls and outcomes as the CLI and every other language. Immutable and safe to share between
 * threads. Every method blocks until its command ends; the network ones wait on SUNAT up to
 * {@link #getHttpTimeout()}, so a UI thread should not call them.
 */
public final class SunatSdkClient {
    /** 90 seconds, the CLI's default. */
    public static final Duration DEFAULT_HTTP_TIMEOUT = Duration.ofSeconds(90);

    private static final long MAX_HTTP_TIMEOUT_SECONDS = 300;
    private static final Map<String, Object> NO_FLAGS = Collections.emptyMap();

    private final Duration httpTimeout;
    private final long timeoutSeconds;

    /** A client that waits on SUNAT up to {@link #DEFAULT_HTTP_TIMEOUT}. */
    public SunatSdkClient() {
        this(DEFAULT_HTTP_TIMEOUT);
    }

    /**
     * A client that waits on SUNAT up to {@code httpTimeout}.
     *
     * @param httpTimeout whole seconds, 1 to 300; the token and the call of the REST commands share it
     * @throws IllegalArgumentException if it is not whole seconds from 1 to 300
     */
    public SunatSdkClient(Duration httpTimeout) {
        Objects.requireNonNull(httpTimeout, "httpTimeout");
        long seconds = httpTimeout.getSeconds();
        if (httpTimeout.getNano() != 0 || seconds < 1 || seconds > MAX_HTTP_TIMEOUT_SECONDS) {
            throw new IllegalArgumentException("httpTimeout must be whole seconds from 1 to 300, not " + httpTimeout);
        }
        this.httpTimeout = httpTimeout;
        this.timeoutSeconds = seconds;
    }

    /**
     * How long a network command waits on SUNAT.
     *
     * @return the timeout given to the constructor
     */
    public Duration getHttpTimeout() {
        return httpTimeout;
    }

    /**
     * The version of the native library loaded, which is the SDK's and equal to this package's. Loads the library.
     *
     * @return {@code 0.1.0-beta.7}, for instance
     * @throws SunatSdkLoadException if the library cannot be loaded
     */
    public static String nativeVersion() {
        return NativeBridge.version();
    }

    /**
     * Renders the unsigned UBL XML of one voucher, daily summary, void communication or despatch advice.
     *
     * @param documentJson the document as JSON, in the shape of docs/input-model.md
     * @return the XML and its file name
     * @throws IllegalArgumentException if the text is not one JSON value
     * @throws SunatSdkException if the SDK rejected the document; the message says which field and why
     */
    public BuildResult build(final String documentJson) {
        Objects.requireNonNull(documentJson, "documentJson");
        try {
            Json.parse(documentJson);
        } catch (Json.MalformedJson e) {
            throw new IllegalArgumentException("documentJson is not one JSON value: " + e.getMessage(), e);
        }
        return NativeBridge.run("build", w -> w.rawValue(documentJson), NO_FLAGS, BuildResult::new);
    }

    /**
     * Signs XML from {@link #build(String)} with XML-DSig (enveloped, RSA-SHA1, as SUNAT requires), with the
     * signature's {@code Id} {@code signatureKG}.
     *
     * @param unsignedXml the XML of {@link BuildResult#getXml()}
     * @param key the certificate and its key
     * @return the signed XML, its DigestValue and SignatureValue
     * @throws SunatSdkException if the key cannot be read or the XML is not the SDK's
     */
    public SignResult sign(String unsignedXml, SigningKey key) {
        return sign(unsignedXml, key, null);
    }

    /**
     * Signs XML from {@link #build(String)} with XML-DSig (enveloped, RSA-SHA1, as SUNAT requires).
     *
     * @param unsignedXml the XML of {@link BuildResult#getXml()}
     * @param key the certificate and its key
     * @param signatureId the signature's {@code Id}; {@code signatureKG} when null
     * @return the signed XML, its DigestValue and SignatureValue
     * @throws SunatSdkException if the key cannot be read or the XML is not the SDK's
     */
    public SignResult sign(final String unsignedXml, SigningKey key, final String signatureId) {
        Objects.requireNonNull(unsignedXml, "unsignedXml");
        Objects.requireNonNull(key, "key");
        Map<String, Object> flags = new LinkedHashMap<String, Object>();
        flags.put("cert", key.getCertificatePath());
        flags.put("key", key.getPrivateKeyPath());
        return NativeBridge.run("sign", w -> object(w, "xml", unsignedXml, "signature_id", signatureId), flags, SignResult::new);
    }

    /**
     * Zips the XML under {@code <name>.xml}, as SUNAT expects it.
     *
     * @param name the file name without extension ({@code 20100066603-01-F001-1})
     * @param xml the signed XML
     * @return the zip and its name
     */
    public ZipResult zip(final String name, final String xml) {
        Objects.requireNonNull(name, "name");
        Objects.requireNonNull(xml, "xml");
        return NativeBridge.run("zip", w -> object(w, "name", name, "xml", xml), NO_FLAGS, ZipResult::new);
    }

    /**
     * Reads a CDR, zip or XML, and classifies its response code (the {@code cdr} command).
     *
     * @param cdr the bytes of the CDR zip or XML
     * @return the CDR read
     * @throws SunatSdkException if the bytes are not a CDR
     */
    public Cdr readCdr(byte[] cdr) {
        Objects.requireNonNull(cdr, "cdr");
        final String encoded = Base64.getEncoder().encodeToString(cdr);
        return NativeBridge.run("cdr", w -> object(w, "cdr_base64", encoded), NO_FLAGS, Cdr::new);
    }

    /**
     * Zips the signed XML and sends it with sendBill. A fault is a result; a send without an answer throws
     * {@link SunatSdkException}, since it may have arrived: send the same XML again, never another number.
     *
     * @param name the file name of {@link BuildResult#getFileName()}
     * @param signedXml the XML of {@link SignResult#getXml()}
     * @param credentials the SOL user and password
     * @param endpoint the billService URL, https only
     * @return the CDR or the fault
     */
    public SendResult send(final String name, final String signedXml, SolCredentials credentials, String endpoint) {
        Objects.requireNonNull(name, "name");
        Objects.requireNonNull(signedXml, "signedXml");
        return NativeBridge.run("send", w -> object(w, "name", name, "xml", signedXml), soap(credentials, endpoint), SendResult::new);
    }

    /**
     * Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary. Store the ticket before
     * anything else. A send without an answer throws, as {@link #send}.
     *
     * @param name {@code RUC-RC|RA-YYYYMMDD-N}
     * @param signedXml the signed summary or void
     * @param credentials the SOL user and password
     * @param endpoint the billService URL, https only
     * @return the ticket or the fault
     */
    public SummaryResult summary(final String name, final String signedXml, SolCredentials credentials, String endpoint) {
        Objects.requireNonNull(name, "name");
        Objects.requireNonNull(signedXml, "signedXml");
        return NativeBridge.run("summary", w -> object(w, "name", name, "xml", signedXml), soap(credentials, endpoint), SummaryResult::new);
    }

    /**
     * Asks getStatus for a summary's or a void's ticket. No answer is {@link Outcome#NO_VERDICT}: ask again, never send
     * again.
     *
     * @param ticket the ticket of {@link SummaryResult#getTicket()}
     * @param credentials the SOL user and password
     * @param endpoint the billService URL, https only
     * @return the ticket's state, a fault or no verdict
     */
    public TicketResult ticket(final String ticket, SolCredentials credentials, String endpoint) {
        Objects.requireNonNull(ticket, "ticket");
        return NativeBridge.run("ticket", w -> object(w, "ticket", ticket), soap(credentials, endpoint), TicketResult::new);
    }

    /**
     * Asks getStatusCdr for one voucher, with its CDR when SUNAT attaches it. No answer is {@link Outcome#NO_VERDICT}.
     *
     * @param ruc the issuer's RUC
     * @param documentType {@code 01}, {@code 03}, {@code 07} or {@code 08}
     * @param series {@code F001}
     * @param number the correlative number
     * @param credentials the SOL user and password
     * @param endpoint the billConsultService URL, https only
     * @return the status, a fault or no verdict
     */
    public StatusResult status(
            final String ruc, final String documentType, final String series, final long number, SolCredentials credentials, String endpoint) {
        requireKey(ruc, documentType, series);
        return NativeBridge.run("status", w -> {
            voucherKey(w, ruc, documentType, series, number);
            w.endObject();
        }, soap(credentials, endpoint), StatusResult::new);
    }

    /**
     * Asks SUNAT's Consulta Integrada whether a voucher (01, 03, 07, 08) exists and in which state, as the issuer and
     * against SUNAT's production endpoints. There is no test environment.
     *
     * @param ruc the issuer's RUC
     * @param documentType {@code 01}, {@code 03}, {@code 07} or {@code 08}
     * @param series {@code F001}
     * @param number the correlative number
     * @param issueDate the issue date
     * @param total the voucher's total (PayableAmount), up to 2 decimals
     * @param credentials the Consulta Integrada pair of «Credenciales de API SUNAT»
     * @return the state, a refused query or no verdict
     */
    public ValidityResult validity(
            String ruc, String documentType, String series, long number, LocalDate issueDate, BigDecimal total, ApiCredentials credentials) {
        return validity(ruc, documentType, series, number, issueDate, total, credentials, null, null, null);
    }

    /**
     * Asks SUNAT's Consulta Integrada whether a voucher (01, 03, 07, 08) exists and in which state: a token, then the
     * query, both inside {@link #getHttpTimeout()}. There is no test environment: this is production.
     *
     * @param ruc the issuer's RUC
     * @param documentType {@code 01}, {@code 03}, {@code 07} or {@code 08}
     * @param series {@code F001}
     * @param number the correlative number
     * @param issueDate the issue date
     * @param total the voucher's total (PayableAmount), up to 2 decimals
     * @param credentials the Consulta Integrada pair of «Credenciales de API SUNAT»
     * @param requesterRuc the RUC that asks; the issuer's when null
     * @param tokenEndpoint replaces the production token base (https only); null keeps it
     * @param endpoint replaces the production query base (https only); null keeps it
     * @return the state, a refused query or no verdict
     */
    public ValidityResult validity(
            final String ruc,
            final String documentType,
            final String series,
            final long number,
            LocalDate issueDate,
            BigDecimal total,
            ApiCredentials credentials,
            final String requesterRuc,
            String tokenEndpoint,
            String endpoint) {
        requireKey(ruc, documentType, series);
        Objects.requireNonNull(issueDate, "issueDate");
        Objects.requireNonNull(total, "total");
        Objects.requireNonNull(credentials, "credentials");
        Map<String, Object> flags = new LinkedHashMap<String, Object>();
        flags.put("client-id", credentials.clientId());
        flags.put("client-secret", credentials.clientSecret());
        flags.put("timeout", timeoutSeconds);
        if (tokenEndpoint != null) {
            flags.put("token-endpoint", tokenEndpoint);
        }
        if (endpoint != null) {
            flags.put("endpoint", endpoint);
        }
        final String date = issueDate.toString();
        final String amount = total.toPlainString();
        return NativeBridge.run("validity", w -> {
            voucherKey(w, ruc, documentType, series, number);
            w.member("issue_date", date);
            w.member("total", amount);
            if (requesterRuc != null) {
                w.member("requester_ruc", requesterRuc);
            }
            w.endObject();
        }, flags, ValidityResult::new);
    }

    /**
     * Sends a signed despatch advice (09) by SUNAT's GRE REST API: a token by password, then the zip with its hash.
     * SUNAT has no test environment for GRE and the endpoints have no default. A send without an answer throws: send
     * the same file again, never the operation with another number.
     *
     * @param name {@code RUC-09-SERIE-NUMERO}
     * @param signedXml the signed guide
     * @param api the GRE pair of «Credenciales de API SUNAT»
     * @param sol the RUC followed by the SOL user, and its password
     * @param tokenEndpoint the token base, https only ({@code https://api-seguridad.sunat.gob.pe/v1/clientessol})
     * @param endpoint the API base, https only ({@code https://api-cpe.sunat.gob.pe/v1})
     * @return the ticket, an error of the API or no verdict
     */
    public GreSendResult greSend(
            final String name, final String signedXml, ApiCredentials api, SolCredentials sol, String tokenEndpoint, String endpoint) {
        Objects.requireNonNull(name, "name");
        Objects.requireNonNull(signedXml, "signedXml");
        return NativeBridge.run("gre-send", w -> object(w, "name", name, "xml", signedXml), gre(api, sol, tokenEndpoint, endpoint), GreSendResult::new);
    }

    /**
     * Asks for the outcome of a sent guide by its ticket (a token, then the query). 98 is in process: ask again with the
     * same ticket.
     *
     * @param ticket the ticket of {@link GreSendResult#getTicket()}
     * @param api the GRE pair of «Credenciales de API SUNAT»
     * @param sol the RUC followed by the SOL user, and its password
     * @param tokenEndpoint the token base, https only
     * @param endpoint the API base, https only
     * @return the ticket's state, an error of the API or no verdict
     */
    public GreTicketResult greTicket(final String ticket, ApiCredentials api, SolCredentials sol, String tokenEndpoint, String endpoint) {
        Objects.requireNonNull(ticket, "ticket");
        return NativeBridge.run("gre-ticket", w -> object(w, "ticket", ticket), gre(api, sol, tokenEndpoint, endpoint), GreTicketResult::new);
    }

    private Map<String, Object> soap(SolCredentials credentials, String endpoint) {
        Objects.requireNonNull(credentials, "credentials");
        Objects.requireNonNull(endpoint, "endpoint");
        Map<String, Object> flags = new LinkedHashMap<String, Object>();
        flags.put("endpoint", endpoint);
        flags.put("user", credentials.user());
        flags.put("password", credentials.password());
        flags.put("timeout", timeoutSeconds);
        return flags;
    }

    private Map<String, Object> gre(ApiCredentials api, SolCredentials sol, String tokenEndpoint, String endpoint) {
        Objects.requireNonNull(api, "api");
        Objects.requireNonNull(sol, "sol");
        Objects.requireNonNull(tokenEndpoint, "tokenEndpoint");
        Objects.requireNonNull(endpoint, "endpoint");
        Map<String, Object> flags = new LinkedHashMap<String, Object>();
        flags.put("client-id", api.clientId());
        flags.put("client-secret", api.clientSecret());
        flags.put("user", sol.user());
        flags.put("password", sol.password());
        flags.put("token-endpoint", tokenEndpoint);
        flags.put("endpoint", endpoint);
        flags.put("timeout", timeoutSeconds);
        return flags;
    }

    private static void requireKey(String ruc, String documentType, String series) {
        Objects.requireNonNull(ruc, "ruc");
        Objects.requireNonNull(documentType, "documentType");
        Objects.requireNonNull(series, "series");
    }

    /** Opens the input object with the voucher's four keys; the caller adds members and closes it. */
    private static void voucherKey(RequestWriter w, String ruc, String documentType, String series, long number) {
        w.startObject();
        w.member("ruc", ruc);
        w.member("document_type", documentType);
        w.member("series", series);
        w.member("number", number);
    }

    /** An object of string members given as name, value, name, value...; a null value is left out. */
    private static void object(RequestWriter w, String... namesAndValues) {
        w.startObject();
        for (int i = 0; i + 1 < namesAndValues.length; i += 2) {
            if (namesAndValues[i + 1] != null) {
                w.member(namesAndValues[i], namesAndValues[i + 1]);
            }
        }
        w.endObject();
    }
}
