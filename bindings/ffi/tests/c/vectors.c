/*
 * The battery from C: for every case named on the command line, `build` its fixture and `sign` the
 * result with the test certificate through sunat_sdk_call, and compare DigestValue and
 * SignatureValue with tests/vectors/<case>.signed.xml. Equal values mean the built XML
 * canonicalizes like the vector, since the digest is taken over that form.
 *
 *   vectors <repository> <case>...
 *
 * With SUNAT_SDK_DYNAMIC (Windows), the library is loaded by name with LoadLibrary and every function
 * is looked up by its undecorated name, as VB6, Visual FoxPro and PowerBuilder do; built with
 * /RTC1 on 32-bit, a calling convention that differs from the header's stops the program.
 */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sunat_sdk.h"

#ifdef SUNAT_SDK_DYNAMIC
#include <windows.h>
typedef int32_t (SUNAT_SDK_CALL *call_fn)(const char *, char **);
typedef void (SUNAT_SDK_CALL *free_fn)(char *);
typedef size_t (SUNAT_SDK_CALL *len_fn)(const char *);
typedef int32_t (SUNAT_SDK_CALL *copy_fn)(const char *, char *, size_t);
typedef const char *(SUNAT_SDK_CALL *version_fn)(void);
typedef uint32_t (SUNAT_SDK_CALL *abi_fn)(void);
static call_fn sdk_call;
static free_fn sdk_free;
static len_fn sdk_len;
static copy_fn sdk_copy;
static version_fn sdk_version;
static abi_fn sdk_abi;

static FARPROC symbol(HMODULE lib, const char *name) {
    FARPROC found = GetProcAddress(lib, name);
    if (!found) {
        fprintf(stderr, "export %s not found (decorated or missing)\n", name);
        exit(1);
    }
    return found;
}

static void load(void) {
    HMODULE lib = LoadLibraryA("sunat_sdk_ffi.dll");
    if (!lib) {
        fprintf(stderr, "LoadLibrary failed: %lu\n", (unsigned long)GetLastError());
        exit(1);
    }
    sdk_call = (call_fn)symbol(lib, "sunat_sdk_call");
    sdk_free = (free_fn)symbol(lib, "sunat_sdk_free");
    sdk_len = (len_fn)symbol(lib, "sunat_sdk_response_len");
    sdk_copy = (copy_fn)symbol(lib, "sunat_sdk_response_copy");
    sdk_version = (version_fn)symbol(lib, "sunat_sdk_version");
    sdk_abi = (abi_fn)symbol(lib, "sunat_sdk_abi_version");
}
#else
#define sdk_call sunat_sdk_call
#define sdk_free sunat_sdk_free
#define sdk_len sunat_sdk_response_len
#define sdk_copy sunat_sdk_response_copy
#define sdk_version sunat_sdk_version
#define sdk_abi sunat_sdk_abi_version
static void load(void) {}
#endif

static void fail(const char *what, const char *detail) {
    fprintf(stderr, "FAIL %s: %.400s\n", what, detail ? detail : "");
    exit(1);
}

static char *read_file(const char *path) {
    FILE *file = fopen(path, "rb");
    long size;
    char *text;
    if (!file) fail("open", path);
    fseek(file, 0, SEEK_END);
    size = ftell(file);
    fseek(file, 0, SEEK_SET);
    text = malloc((size_t)size + 1);
    if (!text || fread(text, 1, (size_t)size, file) != (size_t)size) fail("read", path);
    text[size] = '\0';
    fclose(file);
    return text;
}

/* Concatenates NULL-terminated pieces into a new string. */
static char *join(const char *first, ...) {
    va_list pieces;
    const char *piece;
    size_t total = 0;
    char *out;
    va_start(pieces, first);
    for (piece = first; piece; piece = va_arg(pieces, const char *)) total += strlen(piece);
    va_end(pieces);
    out = malloc(total + 1);
    if (!out) fail("malloc", NULL);
    out[0] = '\0';
    va_start(pieces, first);
    for (piece = first; piece; piece = va_arg(pieces, const char *)) strcat(out, piece);
    va_end(pieces);
    return out;
}

/* The JSON string literal (quotes included) of `"key":` in an object written by serde_json. */
static char *json_string_token(const char *json, const char *key) {
    char *pattern = join("\"", key, "\":\"", NULL);
    const char *start = strstr(json, pattern);
    const char *end;
    char *token;
    free(pattern);
    if (!start) fail("key not in response", key);
    start += strlen(key) + 3;
    for (end = start + 1; *end && *end != '"'; end++) {
        if (*end == '\\') end++;
    }
    if (*end != '"') fail("unterminated string", key);
    token = malloc((size_t)(end - start) + 2);
    memcpy(token, start, (size_t)(end - start) + 1);
    token[end - start + 1] = '\0';
    return token;
}

/* A string value without escapes (base64), quotes removed. */
static char *json_plain_string(const char *json, const char *key) {
    char *token = json_string_token(json, key);
    size_t len = strlen(token);
    memmove(token, token + 1, len - 2);
    token[len - 2] = '\0';
    return token;
}

static char *element_text(const char *xml, const char *element) {
    char *open = join("<", element, ">", NULL);
    const char *start = strstr(xml, open);
    const char *end;
    char *text;
    if (!start) fail("element missing", element);
    start += strlen(open);
    free(open);
    end = strchr(start, '<');
    text = malloc((size_t)(end - start) + 1);
    memcpy(text, start, (size_t)(end - start));
    text[end - start] = '\0';
    return text;
}

/* The path as a JSON string body: Windows paths come with backslashes. */
static char *json_escape(const char *text) {
    char *out = malloc(strlen(text) * 2 + 1);
    char *write = out;
    for (; *text; text++) {
        if (*text == '\\' || *text == '"') *write++ = '\\';
        *write++ = *text;
    }
    *write = '\0';
    return out;
}

/* Calls through the pointer API and, for every other call, re-reads the response through the
 * length-and-copy API that VB6, Visual FoxPro and PowerBuilder use. */
static char *call(const char *request, int32_t expected, const char *what) {
    static int alternate;
    char *response = NULL;
    char *copy;
    size_t len;
    int32_t status = sdk_call(request, &response);
    if (!response) fail(what, "no response");
    len = sdk_len(response);
    copy = malloc(len + 1);
    if (alternate++ % 2) {
        if (sdk_copy(response, copy, len + 1) != SUNAT_SDK_OK) fail(what, "copy");
    } else {
        memcpy(copy, response, len + 1);
    }
    sdk_free(response);
    if (status != expected) fail(what, copy);
    return copy;
}

int main(int argc, char **argv) {
    char *repo, *cert, *key, *bad;
    int i;
    if (argc < 3) {
        fprintf(stderr, "usage: vectors <repository> <case>...\n");
        return 2;
    }
    load();
    if (sdk_abi() != SUNAT_SDK_ABI_VERSION) fail("abi version", sdk_version());
    if (sdk_call("{}", NULL) != SUNAT_SDK_NO_RESPONSE_SLOT) fail("null response slot", NULL);
    bad = call("{\"command\":\"nope\",\"input\":{}}", SUNAT_SDK_ERROR, "unknown command");
    if (strncmp(bad, "{\"error\":\"usage: sunat-sdk", 26) != 0) fail("error shape", bad);
    free(bad);

    repo = argv[1];
    cert = json_escape(join(repo, "/tests/vectors/test-cert/test-cert.pem", NULL));
    key = json_escape(join(repo, "/tests/vectors/test-cert/test-key.pem", NULL));
    for (i = 2; i < argc; i++) {
        const char *name = argv[i];
        char *fixture = read_file(join(repo, "/fixtures/", name, ".json", NULL));
        char *built = call(join("{\"command\":\"build\",\"input\":", fixture, "}", NULL), SUNAT_SDK_OK, name);
        char *xml = json_string_token(built, "xml");
        char *sign = join("{\"command\":\"sign\",\"flags\":{\"cert\":\"", cert, "\",\"key\":\"", key,
                          "\"},\"input\":{\"xml\":", xml, "}}", NULL);
        char *signed_out = call(sign, SUNAT_SDK_OK, name);
        char *vector = read_file(join(repo, "/tests/vectors/", name, ".signed.xml", NULL));
        if (strcmp(json_plain_string(signed_out, "digest_value"), element_text(vector, "ds:DigestValue")) != 0)
            fail(name, "DigestValue differs from the vector");
        if (strcmp(json_plain_string(signed_out, "signature_value"), element_text(vector, "ds:SignatureValue")) != 0)
            fail(name, "SignatureValue differs from the vector");
        free(fixture);
        free(built);
        free(xml);
        free(sign);
        free(signed_out);
        free(vector);
    }
    printf("ok: %d cases through the C ABI (sdk %s, abi %u)\n", argc - 2, sdk_version(), (unsigned)sdk_abi());
    return 0;
}
