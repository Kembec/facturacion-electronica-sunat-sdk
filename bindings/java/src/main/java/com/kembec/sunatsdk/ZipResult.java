package com.kembec.sunatsdk;

import java.util.Map;

/** {@code zip}: the zip SUNAT expects, with the XML inside under {@code <name>.xml}. */
public final class ZipResult extends SdkResult {
    private final String fileName;
    private final byte[] content;

    ZipResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        fileName = Fields.text(value, "file_name");
        byte[] bytes = Fields.optionalBase64(value, "zip_base64");
        if (bytes == null) {
            throw new Fields.ShapeError("`zip_base64` is missing");
        }
        content = bytes;
    }

    /**
     * The zip's name.
     *
     * @return {@code 20123456789-01-F001-1.zip}
     */
    public String getFileName() {
        return fileName;
    }

    /**
     * The zip's bytes.
     *
     * @return a copy of the bytes
     */
    public byte[] getContent() {
        return content.clone();
    }
}
