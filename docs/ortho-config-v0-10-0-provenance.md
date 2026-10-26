# OrthoConfig v0.10.0 documentation provenance

The OrthoConfig guides in this repository are immutable copies of the `v0.10.0`
release documentation. They were imported before application implementation
from the pinned commit below.

## Source

- Repository: <https://github.com/leynos/ortho-config>
- Tag: `v0.10.0`
- Tag object: `996e5a1c9f22ace4f55bf5dcc196d2c0524262bf`
- Peeled commit: `38812c563e3953a490e3efc202609f37e21a0517`
- Tag verification: the fetched tag object and peeled commit matched the values
  supplied by issue #200.

## Imported files

| Upstream path                     | Local path                                     | SHA-256                                                            |
| --------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------ |
| `docs/users-guide.md`             | `docs/ortho-config-users-guide.md`             | `84d1c97b77024224dc9220a12d4b749205d9bb2db54849185da21c969dda5575` |
| `docs/v0-10-0-migration-guide.md` | `docs/ortho-config-v0-10-0-migration-guide.md` | `b5331aa6cfcc07b3dcd188c620db98d0a1a1f13b0c8ef96d88913d701bca25bf` |

Each local file was written from
`git show 38812c563e3953a490e3efc202609f37e21a0517:<upstream-path>` without
text conversion. Repository formatting must preserve these exact bytes.

The v0.10.0 package manifest at the same commit declares version `0.10.0`, Rust
`1.89.0`, and default `serde_json` and TOML features. This was checked against
the immutable upstream manifest; it does not replace the imported release
guides as the integration reference.

## Preserved relative links

The imported Markdown retains its upstream relative links. They point into the
source tree at the pinned commit and are not rewritten because the imported
bytes must remain unchanged. The two imported guide paths were renamed locally:

- `users-guide.md` maps to
  [the local users' guide](ortho-config-users-guide.md).
- `v0-10-0-migration-guide.md` maps to the
  [local migration guide](ortho-config-v0-10-0-migration-guide.md).

The following link targets remain in the upstream repository:

- `adrs/adr-0004-cargo-external-subcommand-entry-point.md` maps to
  [upstream ADR-0004][upstream-adr-0004].
- `cargo-orthohelp-design.md` maps to
  [the upstream cargo-orthohelp design][upstream-help-design].
- `cargo-orthohelp-api-guide.md` maps to
  [the upstream cargo-orthohelp API guide][upstream-help-api].
- `v0-9-0-migration-guide.md` maps to
  [the upstream v0.9.0 migration guide][upstream-v09-migration].
- `../README.md` and `../CHANGELOG.md` map to the
  [upstream README][upstream-readme] and
  [upstream changelog][upstream-changelog].
- `../examples/hello_world/` maps to the
  [upstream Hello World example][upstream-example].
- `../ortho_l10n/README.md` maps to the
  [upstream ortho_l10n README][upstream-l10n-readme].

[upstream-adr-0004]: https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/docs/adrs/adr-0004-cargo-external-subcommand-entry-point.md
[upstream-help-design]: https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/docs/cargo-orthohelp-design.md
[upstream-help-api]: https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/docs/cargo-orthohelp-api-guide.md
[upstream-v09-migration]: https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/docs/v0-9-0-migration-guide.md
[upstream-readme]: https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/README.md
[upstream-changelog]: https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/CHANGELOG.md
[upstream-example]: https://github.com/leynos/ortho-config/tree/38812c563e3953a490e3efc202609f37e21a0517/examples/hello_world
[upstream-l10n-readme]: https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/ortho_l10n/README.md
