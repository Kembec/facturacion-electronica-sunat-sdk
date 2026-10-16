# Releasing

One tag publishes every artifact: the CLI binary on the GitHub release, the npm package, and the PHP package on
Packagist.

1. Set `version` in `[workspace.package]` of `Cargo.toml`. CI fails if `SunatSdk::VERSION` (PHP) or
   `bindings/wasm/npm/package.json` differ from it.
2. Merge to `main` and push the tag `v<version>`. A version with `-` (`0.1.0-beta.3`) is a prerelease and goes to
   npm under the `beta` dist-tag.
3. `release.yml` builds and tests the artifacts, then waits on the `release` environment (only `v*` tags, the owner
   approves).

## Where each registry gets its credentials

| Registry | How it publishes | Credential |
|---|---|---|
| GitHub release | `publish` job | the workflow's `GITHUB_TOKEN` |
| npm (`@kembec/facturacion-electronica-sunat-sdk`) | `publish-npm` job, Trusted Publishing (OIDC) bound to `release.yml` and the `release` environment | none |
| Packagist (`kembec/facturacion-electronica-sunat-sdk`) | reads the tag through the repository's GitHub webhook | none in the workflow |

If npm rejects the OIDC exchange, check the trusted publisher on npmjs.com (package settings): owner `Kembec`,
repository `facturacion-electronica-sunat-sdk`, workflow `release.yml`, environment `release`.
