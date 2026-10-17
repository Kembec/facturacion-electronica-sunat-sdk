package com.kembec.sunatsdk;

import java.util.Map;

/** {@code sign}: the signed XML and the two values of its signature. */
public final class SignResult extends SdkResult {
    private final String xml;
    private final String digestValue;
    private final String signatureValue;

    SignResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        xml = Fields.text(value, "xml");
        digestValue = Fields.text(value, "digest_value");
        signatureValue = Fields.text(value, "signature_value");
    }

    /**
     * The signed XML.
     *
     * @return the XML to send
     */
    public String getXml() {
        return xml;
    }

    /**
     * The {@code DigestValue}, base64: the hash printed on the voucher's representation.
     *
     * @return the digest, base64
     */
    public String getDigestValue() {
        return digestValue;
    }

    /**
     * The {@code SignatureValue}.
     *
     * @return the signature, base64
     */
    public String getSignatureValue() {
        return signatureValue;
    }
}
