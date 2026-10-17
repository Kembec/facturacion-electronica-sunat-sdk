package com.kembec.sunatsdk;

import java.math.BigDecimal;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Collections;
import java.util.List;
import java.util.Map;

/** Typed reads of a response object; a missing or mistyped required member is a {@link ShapeError}. */
final class Fields {
    private Fields() {
    }

    /** The response does not have the shape docs/cli.md promises for its command. */
    static final class ShapeError extends RuntimeException {
        private static final long serialVersionUID = 1L;

        ShapeError(String message) {
            super(message);
        }
    }

    static Map<String, Object> object(Object value) {
        if (!(value instanceof Map)) {
            throw new ShapeError("expected an object");
        }
        @SuppressWarnings("unchecked")
        Map<String, Object> members = (Map<String, Object>) value;
        return members;
    }

    static String text(Map<String, Object> value, String name) {
        String text = optionalText(value, name);
        if (text == null) {
            throw missing(name, "a string");
        }
        return text;
    }

    static String optionalText(Map<String, Object> value, String name) {
        Object field = present(value, name);
        if (field == null) {
            return null;
        }
        if (!(field instanceof String)) {
            throw missing(name, "a string or null");
        }
        return (String) field;
    }

    static Integer optionalInt(Map<String, Object> value, String name) {
        Object field = present(value, name);
        if (field == null) {
            return null;
        }
        if (field instanceof BigDecimal) {
            try {
                return ((BigDecimal) field).intValueExact();
            } catch (ArithmeticException e) {
                // not whole or out of range: the shape error below
            }
        }
        throw missing(name, "a whole number or null");
    }

    static int integer(Map<String, Object> value, String name) {
        Integer number = optionalInt(value, name);
        if (number == null) {
            throw missing(name, "a whole number");
        }
        return number;
    }

    static boolean bool(Map<String, Object> value, String name) {
        Object field = present(value, name);
        if (!(field instanceof Boolean)) {
            throw missing(name, "a boolean");
        }
        return (Boolean) field;
    }

    /** A list of strings; absent or null is empty. */
    static List<String> texts(Map<String, Object> value, String name) {
        Object field = present(value, name);
        if (field == null) {
            return Collections.emptyList();
        }
        if (!(field instanceof List)) {
            throw missing(name, "a list of strings");
        }
        List<String> list = new ArrayList<String>();
        for (Object item : (List<?>) field) {
            if (!(item instanceof String)) {
                throw missing(name, "a list of strings");
            }
            list.add((String) item);
        }
        return Collections.unmodifiableList(list);
    }

    /** The objects of a list; absent or null is empty. */
    static List<Map<String, Object>> objects(Map<String, Object> value, String name) {
        Object field = present(value, name);
        if (field == null) {
            return Collections.emptyList();
        }
        if (!(field instanceof List)) {
            throw missing(name, "a list of objects");
        }
        List<Map<String, Object>> list = new ArrayList<Map<String, Object>>();
        for (Object item : (List<?>) field) {
            list.add(object(item));
        }
        return list;
    }

    static byte[] optionalBase64(Map<String, Object> value, String name) {
        String text = optionalText(value, name);
        if (text == null) {
            return null;
        }
        try {
            return Base64.getDecoder().decode(text);
        } catch (IllegalArgumentException e) {
            throw missing(name, "base64");
        }
    }

    static Map<String, Object> optionalObject(Map<String, Object> value, String name) {
        Object field = present(value, name);
        return field == null ? null : object(field);
    }

    static Map<String, Object> requiredObject(Map<String, Object> value, String name) {
        Map<String, Object> field = optionalObject(value, name);
        if (field == null) {
            throw missing(name, "an object");
        }
        return field;
    }

    static Outcome outcome(Map<String, Object> value, Outcome... allowed) {
        String name = text(value, "outcome");
        for (Outcome outcome : allowed) {
            if (outcome.wireName().equals(name)) {
                return outcome;
            }
        }
        throw new ShapeError("outcome `" + name + "` is not one this command gives");
    }

    static byte[] copy(byte[] bytes) {
        return bytes == null ? null : bytes.clone();
    }

    /** The member's value, or null when it is absent or JSON null. */
    private static Object present(Map<String, Object> value, String name) {
        Object field = value.get(name);
        return field == Json.NULL ? null : field;
    }

    private static ShapeError missing(String name, String expected) {
        return new ShapeError("`" + name + "` is not " + expected);
    }
}
