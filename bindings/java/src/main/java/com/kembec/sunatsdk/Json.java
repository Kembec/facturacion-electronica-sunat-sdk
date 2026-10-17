package com.kembec.sunatsdk;

import java.math.BigDecimal;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * A strict JSON reader (RFC 8259) for the library's responses and for checking a caller's document. The package
 * carries its own so that it adds no JSON library to an application that may already pin another version of one.
 * Objects are {@code Map<String, Object>}, arrays {@code List<Object>}, numbers {@link BigDecimal}, and JSON null is
 * {@link #NULL}.
 */
final class Json {
    /** JSON null, distinct from an absent member. */
    static final Object NULL = new Object() {
        @Override
        public String toString() {
            return "null";
        }
    };

    private static final int MAX_DEPTH = 512;

    private final String text;
    private int pos;

    private Json(String text) {
        this.text = text;
    }

    /** Parses one JSON value with optional surrounding whitespace; anything else is {@link MalformedJson}. */
    static Object parse(String text) {
        Json reader = new Json(text);
        reader.skipWhitespace();
        Object value = reader.value(0);
        reader.skipWhitespace();
        if (reader.pos != text.length()) {
            throw reader.error("unexpected text after the value");
        }
        return value;
    }

    /** Compact JSON of a value read by {@link #parse}; the raw text of a member such as a nested CDR. */
    static String write(Object value) {
        StringBuilder out = new StringBuilder();
        write(value, out);
        return out.toString();
    }

    private static void write(Object value, StringBuilder out) {
        if (value instanceof Map) {
            out.append('{');
            boolean first = true;
            for (Map.Entry<?, ?> member : ((Map<?, ?>) value).entrySet()) {
                if (!first) {
                    out.append(',');
                }
                first = false;
                quote((String) member.getKey(), out);
                out.append(':');
                write(member.getValue(), out);
            }
            out.append('}');
        } else if (value instanceof List) {
            out.append('[');
            boolean first = true;
            for (Object item : (List<?>) value) {
                if (!first) {
                    out.append(',');
                }
                first = false;
                write(item, out);
            }
            out.append(']');
        } else if (value instanceof String) {
            quote((String) value, out);
        } else if (value instanceof BigDecimal) {
            out.append(((BigDecimal) value).toString());
        } else {
            out.append(String.valueOf(value));
        }
    }

    private static void quote(String text, StringBuilder out) {
        out.append('"');
        for (int i = 0; i < text.length(); i++) {
            char c = text.charAt(i);
            if (c == '"' || c == '\\') {
                out.append('\\').append(c);
            } else if (c < 0x20) {
                out.append(String.format("\\u%04x", (int) c));
            } else {
                out.append(c);
            }
        }
        out.append('"');
    }

    /** The text is not one JSON value. */
    static final class MalformedJson extends RuntimeException {
        private static final long serialVersionUID = 1L;

        MalformedJson(String message) {
            super(message);
        }
    }

    private Object value(int depth) {
        if (depth > MAX_DEPTH) {
            throw error("nested deeper than " + MAX_DEPTH);
        }
        if (pos >= text.length()) {
            throw error("a value is missing");
        }
        char c = text.charAt(pos);
        switch (c) {
            case '{':
                return object(depth);
            case '[':
                return array(depth);
            case '"':
                return string();
            case 't':
                return literal("true", Boolean.TRUE);
            case 'f':
                return literal("false", Boolean.FALSE);
            case 'n':
                return literal("null", NULL);
            default:
                if (c == '-' || (c >= '0' && c <= '9')) {
                    return number();
                }
                throw error("unexpected character");
        }
    }

    private Map<String, Object> object(int depth) {
        pos++;
        Map<String, Object> members = new LinkedHashMap<String, Object>();
        skipWhitespace();
        if (peek() == '}') {
            pos++;
            return Collections.unmodifiableMap(members);
        }
        while (true) {
            skipWhitespace();
            if (peek() != '"') {
                throw error("expected a member name");
            }
            String name = string();
            skipWhitespace();
            expect(':');
            skipWhitespace();
            members.put(name, value(depth + 1));
            skipWhitespace();
            char c = next();
            if (c == '}') {
                return Collections.unmodifiableMap(members);
            }
            if (c != ',') {
                throw error("expected ',' or '}'");
            }
        }
    }

    private List<Object> array(int depth) {
        pos++;
        List<Object> items = new ArrayList<Object>();
        skipWhitespace();
        if (peek() == ']') {
            pos++;
            return Collections.unmodifiableList(items);
        }
        while (true) {
            skipWhitespace();
            items.add(value(depth + 1));
            skipWhitespace();
            char c = next();
            if (c == ']') {
                return Collections.unmodifiableList(items);
            }
            if (c != ',') {
                throw error("expected ',' or ']'");
            }
        }
    }

    private String string() {
        pos++;
        StringBuilder out = new StringBuilder();
        while (true) {
            char c = next();
            if (c == '"') {
                return out.toString();
            }
            if (c < 0x20) {
                throw error("control character in a string");
            }
            if (c != '\\') {
                out.append(c);
                continue;
            }
            char e = next();
            switch (e) {
                case '"':
                case '\\':
                case '/':
                    out.append(e);
                    break;
                case 'b':
                    out.append('\b');
                    break;
                case 'f':
                    out.append('\f');
                    break;
                case 'n':
                    out.append('\n');
                    break;
                case 'r':
                    out.append('\r');
                    break;
                case 't':
                    out.append('\t');
                    break;
                case 'u':
                    out.append(hex4());
                    break;
                default:
                    throw error("invalid escape");
            }
        }
    }

    private char hex4() {
        if (pos + 4 > text.length()) {
            throw error("truncated \\u escape");
        }
        int code = 0;
        for (int i = 0; i < 4; i++) {
            int digit = Character.digit(text.charAt(pos++), 16);
            if (digit < 0) {
                throw error("invalid \\u escape");
            }
            code = code * 16 + digit;
        }
        return (char) code;
    }

    private BigDecimal number() {
        int start = pos;
        if (peek() == '-') {
            pos++;
        }
        if (peek() == '0') {
            pos++;
        } else if (isDigit(peek())) {
            digits();
        } else {
            throw error("invalid number");
        }
        if (peek() == '.') {
            pos++;
            if (!isDigit(peek())) {
                throw error("invalid number");
            }
            digits();
        }
        if (peek() == 'e' || peek() == 'E') {
            pos++;
            if (peek() == '+' || peek() == '-') {
                pos++;
            }
            if (!isDigit(peek())) {
                throw error("invalid number");
            }
            digits();
        }
        return new BigDecimal(text.substring(start, pos));
    }

    private void digits() {
        while (isDigit(peek())) {
            pos++;
        }
    }

    private Object literal(String word, Object value) {
        if (!text.startsWith(word, pos)) {
            throw error("unexpected character");
        }
        pos += word.length();
        return value;
    }

    private void skipWhitespace() {
        while (pos < text.length()) {
            char c = text.charAt(pos);
            if (c != ' ' && c != '\t' && c != '\n' && c != '\r') {
                return;
            }
            pos++;
        }
    }

    private void expect(char c) {
        if (next() != c) {
            throw error("expected '" + c + "'");
        }
    }

    private char peek() {
        return pos < text.length() ? text.charAt(pos) : '\0';
    }

    private char next() {
        if (pos >= text.length()) {
            throw error("unexpected end");
        }
        return text.charAt(pos++);
    }

    private static boolean isDigit(char c) {
        return c >= '0' && c <= '9';
    }

    private MalformedJson error(String what) {
        return new MalformedJson(what + " at character " + pos);
    }
}
