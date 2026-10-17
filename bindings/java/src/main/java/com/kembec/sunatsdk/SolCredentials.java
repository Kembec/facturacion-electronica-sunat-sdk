package com.kembec.sunatsdk;

/**
 * The SOL user and password for sendBill, sendSummary, getStatus, getStatusCdr and the GRE token. They reach the
 * native library inside the request, whose bytes are zeroed after the call, and never appear in a result, an
 * exception or {@link #toString()}.
 */
public final class SolCredentials {
    private final String user;
    private final String password;

    /**
     * Creates the credentials.
     *
     * @param user the RUC followed by the SOL user ({@code 20100066603MODDATOS})
     * @param password the SOL password
     */
    public SolCredentials(String user, String password) {
        this.user = SigningKey.notEmpty(user, "user");
        this.password = SigningKey.notEmpty(password, "password");
    }

    String user() {
        return user;
    }

    String password() {
        return password;
    }

    /**
     * Hides both values.
     *
     * @return the class name and {@code [redacted]}
     */
    @Override
    public String toString() {
        return "SolCredentials[redacted]";
    }
}
