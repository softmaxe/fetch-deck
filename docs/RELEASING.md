# Releasing FetchDeck

The release workflow builds and tests the native Apple Silicon binary,
publishes its archive and SHA-256 file, tests the generated Homebrew formula,
and updates `softmaxe/homebrew-tap`.

Before the first release, add a fine-grained personal access token as the
`HOMEBREW_TAP_TOKEN` Actions secret. Limit repository access to
`softmaxe/homebrew-tap` and grant only `Contents: Read and write`.

To release a version:

1. Update the package version in `Cargo.toml` and `Cargo.lock`.
2. Merge the version change into `main` and confirm the release workflow is
   present there.
3. Push a matching tag in complete `vMAJOR.MINOR.PATCH` form.

```sh
release_version=1.0.0
git tag "v${release_version}"
git push origin "v${release_version}"
```

The workflow refuses tags that do not match the package version, existing
GitHub releases, non-native binaries, malformed archives, and Homebrew formula
version rollbacks.
