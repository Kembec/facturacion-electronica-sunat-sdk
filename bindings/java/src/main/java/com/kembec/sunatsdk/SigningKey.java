package com.kembec.sunatsdk;

import java.nio.file.Path;

/**
 * Paths of the PEM certificate and its unencrypted RSA private key. The native library reads the files; their content
 * never passes through Java.
 */
public final class SigningKey {
    private final String certificatePath;
    private final String privateKeyPath;

    /**
     * Creates the key from two file paths.
     *
     * @param certificatePath the certificate, PEM
     * @param privateKeyPath the unencrypted RSA private key, PEM
     */
    public SigningKey(String certificatePath, String privateKeyPath) {
        this.certificatePath = notEmpty(certificatePath, "certificatePath");
        this.privateKeyPath = notEmpty(privateKeyPath, "privateKeyPath");
    }

    /**
     * Creates the key from two file paths.
     *
     * @param certificatePath the certificate, PEM
     * @param privateKeyPath the unencrypted RSA private key, PEM
     */
    public SigningKey(Path certificatePath, Path privateKeyPath) {
        this(text(certificatePath, "certificatePath"), text(privateKeyPath, "privateKeyPath"));
    }

    /**
     * The certificate's path.
     *
     * @return the path as given
     */
    public String getCertificatePath() {
        return certificatePath;
    }

    /**
     * The private key's path.
     *
     * @return the path as given
     */
    public String getPrivateKeyPath() {
        return privateKeyPath;
    }

    /**
     * Hides the paths.
     *
     * @return the class name and {@code [redacted]}
     */
    @Override
    public String toString() {
        return "SigningKey[redacted]";
    }

    static String notEmpty(String value, String name) {
        if (value == null) {
            throw new NullPointerException(name);
        }
        if (value.isEmpty()) {
            throw new IllegalArgumentException(name + " must not be empty");
        }
        return value;
    }

    private static String text(Path path, String name) {
        if (path == null) {
            throw new NullPointerException(name);
        }
        return path.toAbsolutePath().toString();
    }
}
