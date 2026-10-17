# ADR 0001: one C ABI for the languages, not one CLI wrapper per language

Status: proposed (16/10/2026). Decides the base; each language wrapper is its own work.

## Context

Manuel's list of languages the SDK must serve: PHP, C#, VB.NET, Java, VB6, Visual FoxPro, PowerBuilder, Python, Perl,
Objective-C, Swift and Ruby. Today PHP wraps the CLI (`proc_open`), Python and JS use the core in memory through PyO3 and
WebAssembly. A consumer adopts a version only after the whole battery of vectors passes through the build of its own language
(`CLAUDE.md`), so every route is also a thing to test.

## Options

**A. A wrapper per language over the CLI** (the PHP route). Each wrapper starts the `sunat-sdk` binary, writes JSON to
stdin, reads stdout, maps exit 1 to an exception.

**B. A C ABI in process** (`bindings/ffi`): one shared library with a JSON request in, a JSON response out, the CLI's own
dispatch behind it. Each language declares six functions and parses JSON.

**C. The core compiled to WebAssembly and hosted by each language.** Rejected up front: it needs a WASM runtime in every
language (absent in VB6, Visual FoxPro, PowerBuilder, Perl), and the WASM binding has no network I/O by design.

## Measured and checked

- **Cost per call** (this Mac, release, `build` of `01-invoice-taxed-cash`): CLI as a process 2.8 to 4.5 ms; ABI
  0.03 to 0.05 ms, about 80 times less. Irrelevant for one invoice, relevant for a batch or a request path; and a process per
  call is what a restricted PHP host or an IIS pool may forbid.
- **Reach**: VB6, Visual FoxPro and PowerBuilder have no practical `proc_open` with stdin and stdout pipes (VB6 and VFP need
  `WScript.Shell` or `CreateProcess` plumbing, temp files for stdin, and expose the password in a command line or a file).
  All three call a DLL by `Declare` / `DECLARE DLL` / `FUNCTION ... LIBRARY`.
- **Bitness**: VB6 and Visual FoxPro run only as 32-bit processes; PowerBuilder has 32 and 64-bit IDEs and runtimes. A
  child process is no problem for bitness (the CLI is 64-bit); a DLL must match the host, so the i686 Windows build is
  required, with `__stdcall`. `extern "system"` gives that in one source; CI loads it from a 32-bit process.
- **One implementation of the contract**: the dispatch moved from `main.rs` to `bindings/cli/src/lib.rs`; the ABI calls it.
  The 38 vectors pass through the binary (`bindings/cli/tests/build_sign.rs`) and through the ABI from Rust, from C and from
  .NET (`docs/c-abi.md`); the CLI's own 37 tests still pass.

## Decision

**B**, with the CLI kept as it is. The ABI is the base for the languages that load a library; the CLI stays for shell, CI and
any language where a process is the better fit. A language wrapper is then a thin layer (declare six functions, encode UTF-8,
parse JSON, throw on status 1) over a contract that is already tested.

Cost accepted: native binaries per platform (six builds in CI, one distribution problem per registry); the memory contract
is C's (documented in `docs/c-abi.md`, covered by the C test); JSON on both sides, so each language needs a JSON parser
(all have one; VB6 and Visual FoxPro need a small one, which the wrapper brings). Not decided here: how each registry
ships the six binaries (below).

## The 12 languages

| Language | Route | Registry | Platform and bitness | To publish |
|---|---|---|---|---|
| PHP | stays on the CLI; FFI route optional | Packagist (`kembec/facturacion-electronica-sunat-sdk`, exists) | any | nothing new. FFI needs `ext-ffi` enabled, off on most shared hosts: the CLI wrapper stays the default |
| C# | P/Invoke (`StdCall`) | NuGet | .NET Framework and .NET, x86 and x64, Linux and macOS via `runtimes/<rid>/native` | NuGet account, API key, reserve the id prefix; package signing optional |
| VB.NET | same NuGet package as C# | NuGet | same | none beyond C#; a VB.NET sample in its README |
| Java | JNA or the FFM API (Java 22+) over the library | Maven Central | all six, natives inside the jar | namespace verification at the Central Portal (`io.github.kembec` by a repository check, or a domain by DNS TXT), user token, GPG signing key, sources and javadoc jars |
| VB6 | `Declare` to the DLL | none | Windows, 32-bit only | nothing to publish: a zip on the GitHub release with the i686 DLL, a `.bas` module and a sample |
| Visual FoxPro | `DECLARE DLL` | none | Windows, 32-bit only | same: zip on the release with a `.prg` |
| PowerBuilder | external function declaration | none | Windows, 32 and 64-bit | same: zip with a `.srf`; i686 or x86_64 by IDE bitness |
| Python | stays on PyO3 wheels (core in process, no ABI) | PyPI (exists) | wheels per platform | nothing new. Moving it to the ABI would trade a typed, tested in-process binding for ctypes: no |
| Perl | `FFI::Platypus` over the library | CPAN | Linux, macOS, Windows; build needs the library on the machine | PAUSE account (requested by hand, not instant); native distribution via Alien-style packaging |
| Objective-C | links the static library or loads the dylib | CocoaPods | macOS and iOS | CocoaPods trunk stops accepting new versions on 2 December 2026 (blog.cocoapods.org, read 16/10/2026): a podspec served from the Git repository (no trunk) is the only route that outlives it |
| Swift | Swift package with a binary target or system library over the header | Swift Package Manager (a Git tag, no registry) | macOS, iOS, Linux | nothing to register: a public tag; a binary target needs an `.xcframework` zip with its checksum on the release |
| Ruby | `ffi` gem | RubyGems | all six, platform gems | RubyGems account with MFA, API key |

PHP and Python: they do not need to move. PHP's CLI wrapper works on hosts where FFI is disabled and Python's wheels are
in-process already; both keep passing the same battery through their own build. The ABI is for the languages that had no
route.

## Consequences

- `bindings/ffi` and its CI (`.github/workflows/ffi.yml`) are the base; wrappers are separate work, one per language, each
  with the 38 vectors through its own build.
- Version rule: `sunat_sdk_abi_version()` changes only on an incompatible change; a wrapper checks it at load.
- Distribution of native binaries is the open cost: the release job that attaches them is not part of this change
  (`docs/releasing.md` is untouched; nothing is published).
- Verified later, not now: that `extern "system"` exports undecorated names on i686 MSVC (CI checks the export table and
  loads by name), and the signing of the DLL for hosts that require it.
