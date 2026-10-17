package com.kembec.sunatsdk;

/**
 * Client id and secret of «Credenciales de API SUNAT» (created in SOL): the Consulta Integrada pair for
 * {@code validity}, or the GRE pair for {@code greSend} and {@code greTicket}. Not the SOL user.
 */
public final class ApiCredentials {
    private final String clientId;
    private final String clientSecret;

    /**
     * Creates the credentials.
     *
     * @param clientId the client id
     * @param clientSecret the client secret
     */
    public ApiCredentials(String clientId, String clientSecret) {
        this.clientId = SigningKey.notEmpty(clientId, "clientId");
        this.clientSecret = SigningKey.notEmpty(clientSecret, "clientSecret");
    }

    String clientId() {
        return clientId;
    }

    String clientSecret() {
        return clientSecret;
    }

    /**
     * Hides both values.
     *
     * @return the class name and {@code [redacted]}
     */
    @Override
    public String toString() {
        return "ApiCredentials[redacted]";
    }
}
