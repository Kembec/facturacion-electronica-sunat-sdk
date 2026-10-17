package com.kembec.sunatsdk;

import java.util.Arrays;

/**
 * The request of one call as NUL-terminated UTF-8 JSON, written straight into bytes. The SOL password and the API
 * secret pass through it, so every array it outgrows and the last one are zeroed (docs/c-abi.md: the caller clears
 * the request after the call): no copy of a secret is left for the garbage collector to free later.
 */
final class RequestWriter implements AutoCloseable {
    private byte[] buffer = new byte[4096];
    private int length;
    private boolean needsComma;

    void startObject() {
        separate();
        put('{');
        needsComma = false;
    }

    void endObject() {
        put('}');
        needsComma = true;
    }

    /** A member name; the next value belongs to it. */
    void name(String name) {
        separate();
        string(name);
        put(':');
        needsComma = false;
    }

    void stringValue(String value) {
        separate();
        string(value);
        needsComma = true;
    }

    void numberValue(long value) {
        separate();
        ascii(Long.toString(value));
        needsComma = true;
    }

    /** A value already checked to be one JSON value ({@link Json#parse}), written as the caller wrote it. */
    void rawValue(String json) {
        separate();
        utf8(json.trim(), false);
        needsComma = true;
    }

    void member(String name, String value) {
        name(name);
        stringValue(value);
    }

    void member(String name, long value) {
        name(name);
        numberValue(value);
    }

    /** The bytes with their terminating NUL; valid until {@link #close()}. */
    byte[] terminated() {
        put((byte) 0);
        return buffer;
    }

    /** Bytes written, the NUL included once {@link #terminated()} ran. */
    int length() {
        return length;
    }

    @Override
    public void close() {
        Arrays.fill(buffer, (byte) 0);
        length = 0;
    }

    private void separate() {
        if (needsComma) {
            put(',');
        }
    }

    private void string(String value) {
        put('"');
        utf8(value, true);
        put('"');
    }

    private void ascii(String text) {
        for (int i = 0; i < text.length(); i++) {
            put(text.charAt(i));
        }
    }

    /** UTF-8 of {@code text}; inside a string, the escapes JSON requires. An unpaired surrogate is refused. */
    private void utf8(String text, boolean escape) {
        for (int i = 0; i < text.length(); i++) {
            char c = text.charAt(i);
            if (escape && (c == '"' || c == '\\')) {
                put('\\');
                put(c);
            } else if (escape && c < 0x20) {
                ascii(String.format("\\u%04x", (int) c));
            } else if (c < 0x80) {
                put(c);
            } else if (c < 0x800) {
                put((byte) (0xC0 | (c >> 6)));
                put((byte) (0x80 | (c & 0x3F)));
            } else if (Character.isHighSurrogate(c) && i + 1 < text.length() && Character.isLowSurrogate(text.charAt(i + 1))) {
                int code = Character.toCodePoint(c, text.charAt(++i));
                put((byte) (0xF0 | (code >> 18)));
                put((byte) (0x80 | ((code >> 12) & 0x3F)));
                put((byte) (0x80 | ((code >> 6) & 0x3F)));
                put((byte) (0x80 | (code & 0x3F)));
            } else if (Character.isSurrogate(c)) {
                throw new IllegalArgumentException("text with an unpaired surrogate at character " + i + " is not valid UTF-16");
            } else {
                put((byte) (0xE0 | (c >> 12)));
                put((byte) (0x80 | ((c >> 6) & 0x3F)));
                put((byte) (0x80 | (c & 0x3F)));
            }
        }
    }

    private void put(char ascii) {
        put((byte) ascii);
    }

    private void put(byte b) {
        if (length == buffer.length) {
            byte[] grown = Arrays.copyOf(buffer, buffer.length * 2);
            Arrays.fill(buffer, (byte) 0);
            buffer = grown;
        }
        buffer[length++] = b;
    }
}
