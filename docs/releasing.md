# Releasing

One tag publishes every artifact: the CLI binary on the GitHub release, the npm package, the Python package on
PyPI, and the PHP package on Packagist. The five library crates are ready for crates.io, but their publish job is
off until the owner has a crates.io account and token ([below](#cratesio)).

1. Set `version` in `[workspace.package]` of `Cargo.toml`. CI fails if `SunatSdk::VERSION` (PHP) or
   `bindings/wasm/npm/package.json` differ from it. The Python package takes it from there as PEP 440
   (`0.1.0-beta.4` is `0.1.0b4`); `bindings/python/scripts/check_version.py` fails if a wheel, the sdist or the tag
   differ. Set the same version, with `=` in front, on the five crates of `[workspace.dependencies]`:
   `crates/scripts/check-publish.sh` fails if they differ. In `CHANGELOG.md`, turn `## [Unreleased]` into
   `## [<version>] - YYYY-MM-DD` above a new empty `## [Unreleased]`, and update the compare links at the end: the
   `changelog` job of `release.yml` fails, on the tag and on the pull requests that run `release.yml`, if the
   version has no such section, and every publish job waits on it.
2. Merge to `main` and push the tag `v<version>`. A version with `-` (`0.1.0-beta.4`) is a prerelease and goes to
   npm under the `beta` dist-tag.
3. `release.yml` builds and tests the artifacts, then waits on the `release` environment (only `v*` tags, the owner
   approves).

## Versions

The public contract is the input model ([`input-model.md`](input-model.md)), the CLI's JSON and exit codes
([`cli.md`](cli.md)), the public API of each binding, and the C ABI ([`c-abi.md`](c-abi.md)). Anything else (internal
crate modules, error message wording, the repository's scripts) can change in any release.

- **While the version is `0.1.0-beta.N`**, a change that breaks the contract may ship in a beta, but it goes in
  `CHANGELOG.md` as **Breaking**, with how to migrate.
- **The first stable version** (`1.0.0`) requires two betas in a row without a **Breaking** entry.
- **From `1.0.0`**, semantic versioning: MAJOR breaks the contract; MINOR adds to it (a document type, an optional
  input field, an operation); PATCH fixes without changing it.
- Adding an error code or an output field does not break the contract; removing one, or changing what it means, does.
  So a consumer ignores fields and codes it does not know.

Every change a consumer would notice gets one line in `CHANGELOG.md` under `## [Unreleased]`
([Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/): Added, Changed, Fixed, Removed), with its pull
request; one line per change, not per commit.

## Where each registry gets its credentials

| Registry | How it publishes | Credential |
|---|---|---|
| GitHub release | `publish` job | the workflow's `GITHUB_TOKEN` |
| npm (`@kembec/facturacion-electronica-sunat-sdk`) | `publish-npm` job, Trusted Publishing (OIDC) bound to `release.yml` and the `release` environment | none |
| PyPI (`facturacion-electronica-sunat-sdk`) | `publish-pypi` job | `PYPI_API_TOKEN` in the `release` environment |
| Packagist (`kembec/facturacion-electronica-sunat-sdk`) | reads the tag through the repository's GitHub webhook | none in the workflow |
| crates.io (`kembec-sunat-model`, `kembec-sunat-transport`, `kembec-sunat-sdk`, `kembec-sunat-xmldsig`, `kembec-sunat-ubl`) | `publish-crates` job, off | `CARGO_REGISTRY_TOKEN` in the `release` environment, not created yet |

If npm rejects the OIDC exchange, check the trusted publisher on npmjs.com (package settings): owner `Kembec`,
repository `facturacion-electronica-sunat-sdk`, workflow `release.yml`, environment `release`.

To move PyPI to Trusted Publishing: add a trusted publisher on pypi.org (project `facturacion-electronica-sunat-sdk`,
owner `Kembec`, repository `facturacion-electronica-sunat-sdk`, workflow `release.yml`, environment `release`), drop the
`password` input from `publish-pypi` and delete `PYPI_API_TOKEN`.

## crates.io

`crates.yml` proves, on every push that touches them, what `cargo publish` would upload: a dry-run of each crate in
dependency order, the docs.rs build, the declared `rust-version`, and the vector battery compiled from the `.crate`
files (`crates/scripts/test-package.sh`). The `crates` job of `release.yml` repeats it on the tag. The packages carry
the library, its tests' code, README, `LICENSE` and `NOTICE`; the data the tests read stays in the repository. The folders under `crates/` keep their `sunat-*` names; the packages are
`kembec-sunat-*`, and the workspace and the bindings depend on them with `package = "kembec-sunat-…"` under the short key.
The library names follow the package: `kembec_sunat_sdk`, and so on.

To turn the publish on:

1. The owner signs in to crates.io with GitHub and creates an API token with the `publish-new` and `publish-update`
   scopes, limited to the crate pattern `kembec-sunat-*`.
2. Store it as `CARGO_REGISTRY_TOKEN` in the `release` environment and drop `false &&` from the `if` of
   `publish-crates`. The next `v*` tag publishes the five crates in dependency order; `v0.1.0-beta.4` was tagged
   before they were ready.

Once the crates exist, Trusted Publishing can replace the token: on each crate's settings on crates.io add owner
`Kembec`, repository `facturacion-electronica-sunat-sdk`, workflow `release.yml`, environment `release`; give
`publish-crates` `id-token: write`, get the token from `rust-lang/crates-io-auth-action`, and delete
`CARGO_REGISTRY_TOKEN`.
