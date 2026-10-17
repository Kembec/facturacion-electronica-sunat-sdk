package com.kembec.sunatsdk;

import com.sun.jna.Library;
import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Platform;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;
import com.sun.jna.win32.StdCallLibrary;
import java.io.File;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import java.util.function.BiFunction;
import java.util.function.Consumer;

/**
 * One call of the C ABI (docs/c-abi.md) through JNA: the library loaded once, the request in, the response read,
 * released and mapped.
 */
final class NativeBridge {
    /** The system property that points to a library file to load instead of the one inside the jar. */
    static final String LIBRARY_PROPERTY = "kembec.sunatsdk.library";

    private static final int ABI_VERSION = 1;
    private static final int STATUS_OK = 0;
    private static final int STATUS_ERROR = 1;
    private static final String RESOURCE_DIR = "/com/kembec/sunatsdk/native/";
    private static final String SUPPORTED = "win32-x86, win32-x86-64, linux-x86-64, linux-aarch64, darwin-x86-64 and darwin-aarch64";

    private static final Object GATE = new Object();
    private static volatile Abi abi;
    private static volatile String version;

    private NativeBridge() {
    }

    /** The six functions of sunat_sdk.h this package calls; {@code uint32_t} is read as {@code int}. */
    interface Abi extends Library {
        int sunat_sdk_call(Pointer request, PointerByReference response);

        void sunat_sdk_free(Pointer response);

        Pointer sunat_sdk_version();

        int sunat_sdk_abi_version();
    }

    static String version() {
        String known = version;
        if (known == null) {
            known = readText(loaded().sunat_sdk_version(), null);
            version = known;
        }
        return known;
    }

    /**
     * Runs {@code command} and maps its response with {@code read}. Status 1 is a {@link SunatSdkException} with the
     * library's message; a response that is not the shape the contract promises is one too.
     */
    static <T> T run(
            String command,
            Consumer<RequestWriter> input,
            Map<String, Object> flags,
            BiFunction<Map<String, Object>, String, T> read) {
        Abi library = loaded();
        int status;
        String text;
        try (RequestWriter request = new RequestWriter()) {
            write(request, command, input, flags);
            byte[] bytes = request.terminated();
            int length = request.length();
            Memory memory = new Memory(length);
            try {
                memory.write(0, bytes, 0, length);
                PointerByReference response = new PointerByReference();
                status = library.sunat_sdk_call(memory, response);
                text = take(library, command, response.getValue());
            } finally {
                memory.clear();
                memory.close();
            }
        }
        Map<String, Object> root;
        try {
            root = Fields.object(Json.parse(text));
        } catch (Json.MalformedJson | Fields.ShapeError e) {
            throw new SunatSdkException(command, "the library's response is not a JSON object: " + e.getMessage(), e);
        }
        try {
            if (status == STATUS_ERROR) {
                throw new SunatSdkException(command, Fields.text(root, "error"));
            }
            if (status != STATUS_OK) {
                throw new SunatSdkException(command, "the library answered status " + status);
            }
            return read.apply(root, text);
        } catch (Fields.ShapeError e) {
            throw new SunatSdkException(command, "unexpected response: " + e.getMessage(), e);
        }
    }

    static Abi loaded() {
        Abi known = abi;
        if (known != null) {
            return known;
        }
        synchronized (GATE) {
            if (abi == null) {
                abi = load();
            }
            return abi;
        }
    }

    private static void write(RequestWriter request, String command, Consumer<RequestWriter> input, Map<String, Object> flags) {
        request.startObject();
        request.member("command", command);
        // The input (an XML of megabytes, at most) goes before the flags, so the buffer has done its growing before a
        // password is written into it.
        request.name("input");
        input.accept(request);
        if (!flags.isEmpty()) {
            request.name("flags");
            request.startObject();
            for (Map.Entry<String, Object> flag : flags.entrySet()) {
                if (flag.getValue() instanceof Number) {
                    request.member(flag.getKey(), ((Number) flag.getValue()).longValue());
                } else {
                    request.member(flag.getKey(), (String) flag.getValue());
                }
            }
            request.endObject();
        }
        request.endObject();
    }

    private static String take(Abi library, String command, Pointer response) {
        if (response == null) {
            throw new SunatSdkException(command, "the library stored no response");
        }
        try {
            return readText(response, command);
        } finally {
            library.sunat_sdk_free(response);
        }
    }

    /** A NUL-terminated string of the library, decoded as strict UTF-8. */
    private static String readText(Pointer text, String command) {
        long length = text.indexOf(0, (byte) 0);
        if (length > Integer.MAX_VALUE) {
            throw new SunatSdkException(command, "the library's response is larger than 2 GB");
        }
        byte[] bytes = text.getByteArray(0, (int) length);
        try {
            return StandardCharsets.UTF_8.newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(ByteBuffer.wrap(bytes))
                    .toString();
        } catch (CharacterCodingException e) {
            throw new SunatSdkException(command, "the library's response is not UTF-8", e);
        }
    }

    private static Abi load() {
        File file = locate();
        Map<String, Object> options = new HashMap<String, Object>();
        // sunat_sdk.h: __stdcall on 32-bit Windows (SUNAT_SDK_CALL), the C convention everywhere else.
        if (Platform.isWindows() && !Platform.is64Bit()) {
            options.put(Library.OPTION_CALLING_CONVENTION, StdCallLibrary.STDCALL_CONVENTION);
        }
        Abi library;
        int found;
        try {
            library = Native.load(file.getAbsolutePath(), Abi.class, options);
            found = library.sunat_sdk_abi_version();
        } catch (UnsatisfiedLinkError e) {
            throw new SunatSdkLoadException("could not load " + file + " (" + platform() + "): " + e.getMessage(), e);
        }
        if (found != ABI_VERSION) {
            throw new SunatSdkLoadException("the native library speaks ABI " + found + " and this package ABI " + ABI_VERSION
                    + ": the package and its native library come from different versions");
        }
        return library;
    }

    /** The library named by {@link #LIBRARY_PROPERTY}, or the jar's own for this platform, extracted by JNA. */
    private static File locate() {
        String override = System.getProperty(LIBRARY_PROPERTY);
        if (override != null && !override.isEmpty()) {
            File file = new File(override);
            if (!file.isFile()) {
                throw new SunatSdkLoadException(LIBRARY_PROPERTY + " names " + override + ", which is not a file");
            }
            return file;
        }
        String resource = RESOURCE_DIR + Platform.RESOURCE_PREFIX + "/" + System.mapLibraryName("sunat_sdk_ffi");
        if (NativeBridge.class.getResource(resource) == null) {
            throw new SunatSdkLoadException("this package has no native library for " + platform() + "; it carries "
                    + SUPPORTED + ". Point the system property " + LIBRARY_PROPERTY + " to a build of bindings/ffi");
        }
        ClassLoader loader = NativeBridge.class.getClassLoader();
        try {
            return Native.extractFromResourcePath(resource, loader != null ? loader : ClassLoader.getSystemClassLoader());
        } catch (IOException e) {
            throw new SunatSdkLoadException("could not extract " + resource + " to a temporary file (set jna.tmpdir to a"
                    + " writable directory where files may be executed): " + e.getMessage(), e);
        }
    }

    private static String platform() {
        return Platform.RESOURCE_PREFIX + ", " + System.getProperty("os.name") + " " + System.getProperty("os.arch")
                + ", Java " + System.getProperty("java.version");
    }
}
