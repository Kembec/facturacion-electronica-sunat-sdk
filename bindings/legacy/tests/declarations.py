"""The DLL declarations of a VB6 module, a Visual FoxPro program or a PowerBuilder object, as C.

    python declarations.py <vb6|vfp|powerbuilder> <source file> <sunat_sdk.h> <output .h>

None of those IDEs runs in CI, so the battery (battery.c) calls the DLL through the declarations the
zip ships, translated type by type into C function pointers: an argument of the wrong size or a
missing NUL slot shows in the 38 vectors, and on 32 bits /RTC1 stops a call whose stack differs
from __stdcall's. The output also asserts, at compile time, that each type has the size of the
parameter in sunat_sdk.h for the bitness being compiled.

A declaration that would compile in its IDE but cannot work is refused here: a VB6 `As String` or a
PowerBuilder string argument (converted to the ANSI code page or UTF-16, not UTF-8), a string result
read as UTF-16, a missing function.
"""

import re
import sys

FUNCTIONS = ("call", "response_len", "response_copy", "free", "version", "abi_version")

# Scalar types of each language as C types. A by-reference parameter is a pointer to them.
SCALARS = {
    "vb6": {"long": "int32_t", "integer": "int16_t", "byte": "uint8_t"},
    "vfp": {"integer": "int32_t", "long": "int32_t", "short": "int16_t", "string": "char"},
    "powerbuilder": {"long": "int32_t", "ulong": "uint32_t", "longptr": "intptr_t", "int": "int16_t",
                     "uint": "uint16_t", "blob": "uint8_t"},
}


class Refused(Exception):
    pass


def param(ctype, by_ref):
    return {"ctype": ctype, "by_ref": by_ref}


def scalar(lang, name, where):
    try:
        return SCALARS[lang][name.lower()]
    except KeyError:
        raise Refused(f"{where}: type {name} has no C equivalent here") from None


def parse_vb6(text):
    pattern = re.compile(
        r'^\s*(?:Private\s+|Public\s+)?Declare\s+(Function|Sub)\s+(sunat_sdk_\w+)\s+Lib\s+"([^"]+)"'
        r'(?:\s+Alias\s+"([^"]+)")?\s*\(([^)]*)\)(?:\s+As\s+(\w+))?\s*$', re.I)
    found = {}
    for line in text.splitlines():
        match = pattern.match(line)
        if not match:
            continue
        kind, name, _lib, alias, params, result = match.groups()
        args = []
        for raw in filter(None, (p.strip() for p in params.split(","))):
            parts = re.match(r"^(?:(ByVal|ByRef)\s+)?(\w+)\s+As\s+(\w+)$", raw, re.I)
            if not parts:
                raise Refused(f"{name}: cannot read parameter '{raw}'")
            mode, _arg, vtype = parts.groups()
            if vtype.lower() == "string":
                raise Refused(f"{name}: As String goes to the DLL in the ANSI code page; pass UTF-8 bytes As Byte")
            # VB6 passes by reference unless ByVal is written.
            args.append(param(scalar("vb6", vtype, name), (mode or "ByRef").lower() == "byref"))
        if kind.lower() == "sub":
            ret = "void"
        elif result is None or result.lower() == "string":
            raise Refused(f"{name}: a Function needs a numeric As type")
        else:
            ret = scalar("vb6", result, name)
        found[name] = {"symbol": alias or name, "ret": ret, "args": args}
    return found


def parse_vfp(text):
    text = re.sub(r";[ \t]*\r?\n", " ", text)
    pattern = re.compile(
        r"^\s*DECLARE\s+(?:(INTEGER|LONG|SHORT|STRING)\s+)?(sunat_sdk_\w+)\s+IN\s+(\([^)]*\)|\S+)"
        r"(?:\s+AS\s+\w+)?\s*(.*?)\s*$", re.I)
    found = {}
    for line in text.splitlines():
        match = pattern.match(line)
        if not match:
            continue
        result, name, _lib, params = match.groups()
        args = []
        for raw in filter(None, (p.strip() for p in params.split(","))):
            parts = re.match(r"^(\w+)\s*(@)?\s*\w*$", raw)
            if not parts:
                raise Refused(f"{name}: cannot read parameter '{raw}'")
            vtype, at = parts.groups()
            ctype = scalar("vfp", vtype, name)
            # A STRING is always passed as a pointer to its bytes, with or without @.
            args.append(param(ctype, bool(at) or vtype.lower() == "string"))
        if result is None:
            ret = "void"
        elif result.lower() == "string":
            ret = "const char *"
        else:
            ret = scalar("vfp", result, name)
        found[name] = {"symbol": name, "ret": ret, "args": args}
    return found


def parse_powerbuilder(text):
    pattern = re.compile(
        r'^\s*(function|subroutine)\s+(?:(\w+)\s+)?(sunat_sdk_\w+)\s*\(([^)]*)\)\s+library\s+"([^"]+)"'
        r'(?:\s+alias\s+for\s+"([^"]+)")?\s*$', re.I)
    found = {}
    for line in text.splitlines():
        match = pattern.match(line)
        if not match:
            continue
        kind, result, name, params, _lib, alias = match.groups()
        symbol, _, flavor = (alias or name).partition(";")
        args = []
        for raw in filter(None, (p.strip() for p in params.split(","))):
            parts = re.match(r"^(?:(ref)\s+)?(\w+)\s+\w+$", raw, re.I)
            if not parts:
                raise Refused(f"{name}: cannot read parameter '{raw}'")
            ref, vtype = parts.groups()
            if vtype.lower() == "string":
                raise Refused(f"{name}: a string argument goes as UTF-16 or ANSI; pass UTF-8 bytes as ref blob")
            if vtype.lower() == "blob" and not ref:
                raise Refused(f"{name}: a blob reaches an external function only by reference")
            args.append(param(scalar("powerbuilder", vtype, name), bool(ref)))
        if kind.lower() == "subroutine":
            ret = "void"
        elif result.lower() == "string":
            if flavor.lower() != "ansi":
                raise Refused(f"{name}: a string result without ;Ansi is read as UTF-16")
            ret = "const char *"
        else:
            ret = scalar("powerbuilder", result, name)
        found[name] = {"symbol": symbol, "ret": ret, "args": args}
    return found


def parse_header(text):
    pattern = re.compile(r"^(.+?)\s*SUNAT_SDK_CALL\s+(sunat_sdk_\w+)\((.*)\);\s*$")
    found = {}
    for line in text.splitlines():
        match = pattern.match(line)
        if not match:
            continue
        ret, name, params = match.groups()
        args = []
        for raw in params.split(","):
            raw = raw.strip()
            if raw in ("", "void"):
                continue
            args.append(re.sub(r"\s*\b\w+$", "", raw).strip())
        found[name] = {"ret": ret.strip(), "args": args}
    return found


def pointee(ctype):
    """`char **` -> `char *`, `const char *` -> `const char`; None if not a pointer."""
    if not ctype.endswith("*"):
        return None
    return ctype[:-1].rstrip()


def emit(lang, source, declared, header):
    out = [f"/* Generated by bindings/legacy/tests/declarations.py from {source} ({lang}). Do not edit. */",
           f'#define LEGACY_LANGUAGE "{lang}"']
    asserts = []
    for short in FUNCTIONS:
        name = "sunat_sdk_" + short
        if name not in declared:
            raise Refused(f"{name} is not declared in {source}")
        ours, theirs = declared[name], header[name]
        if len(ours["args"]) != len(theirs["args"]):
            raise Refused(f"{name}: {len(ours['args'])} parameters, sunat_sdk.h has {len(theirs['args'])}")
        out.append(f"typedef {ours['ret']} RET_{short};")
        if (ours["ret"] == "void") != (theirs["ret"] == "void"):
            raise Refused(f"{name}: the result is {ours['ret']}, sunat_sdk.h says {theirs['ret']}")
        if ours["ret"] != "void":
            asserts.append((f"sizeof(RET_{short}) == sizeof({theirs['ret']})", f"{name} result"))
        names = []
        for index, (arg, expected) in enumerate(zip(ours["args"], theirs["args"])):
            label = f"{name} parameter {index + 1}"
            if arg["by_ref"]:
                target = pointee(expected)
                if target is None:
                    raise Refused(f"{label}: passed by reference, sunat_sdk.h takes {expected} by value")
                out.append(f"typedef {arg['ctype']} PTE_{short}_{index};")
                out.append(f"typedef PTE_{short}_{index} *ARG_{short}_{index};")
                asserts.append((f"sizeof(PTE_{short}_{index}) == sizeof({target})", f"{label}, what it points to"))
            else:
                out.append(f"typedef {arg['ctype']} ARG_{short}_{index};")
            asserts.append((f"sizeof(ARG_{short}_{index}) == sizeof({expected})", label))
            names.append(f"ARG_{short}_{index}")
        # VB6 Declare, VFP DECLARE and PowerBuilder external functions all call __stdcall: that is
        # the language's convention, not the header's, so it is written here and not taken from it.
        out.append(f"typedef RET_{short} (LEGACY_STDCALL *FN_{short})({', '.join(names) or 'void'});")
        out.append(f'static const char SYMBOL_{short}[] = "{ours["symbol"]}";')
    for condition, label in asserts:
        out.append(f'_Static_assert({condition}, "{lang}: {label} differs in size from sunat_sdk.h");')
    return "\n".join(out) + "\n"


def main():
    if len(sys.argv) != 5 or sys.argv[1] not in SCALARS:
        sys.exit(__doc__)
    lang, source, header_path, output = sys.argv[1:]
    with open(source, encoding="ascii") as file:
        text = file.read()
    with open(header_path, encoding="utf-8") as file:
        header = parse_header(file.read())
    parser = {"vb6": parse_vb6, "vfp": parse_vfp, "powerbuilder": parse_powerbuilder}[lang]
    try:
        generated = emit(lang, source, parser(text), header)
    except Refused as refused:
        sys.exit(f"{source}: {refused}")
    with open(output, "w", encoding="ascii", newline="\n") as file:
        file.write(generated)
    print(f"{source}: {len(FUNCTIONS)} declarations -> {output}")


if __name__ == "__main__":
    main()
