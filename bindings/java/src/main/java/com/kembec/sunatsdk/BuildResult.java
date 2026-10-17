package com.kembec.sunatsdk;

import java.util.Map;

/** {@code build}: the unsigned UBL XML and the SUNAT file name without extension. */
public final class BuildResult extends SdkResult {
    private final String fileName;
    private final String xml;

    BuildResult(Map<String, Object> value, String rawJson) {
        super(rawJson);
        fileName = Fields.text(value, "file_name");
        xml = Fields.text(value, "xml");
    }

    /**
     * The file name without extension.
     *
     * @return {@code 20123456789-01-F001-1}, or {@code RUC-RC|RA-YYYYMMDD-N} for summaries and voids
     */
    public String getFileName() {
        return fileName;
    }

    /**
     * The unsigned XML.
     *
     * @return the XML, to pass to {@link SunatSdkClient#sign(String, SigningKey)}
     */
    public String getXml() {
        return xml;
    }
}
