# Vendored Impactor components

Source: https://github.com/claration/Impactor, commit
`b13d99559febd63e3710dfdd0ef8cdd78d10ae13`. Only `plume_core`,
`omnisette`, and the upstream-patched `apple-codesign` dependency are included.
The component license files are retained alongside their sources; the root
Impactor license is in `LICENSE`.

Local changes: the workspace is reduced to these components; `omnisette` enables
the Chrono clock feature, defaults to local-only instead of falling back to a
remote service, requires explicit HTTPS opt-in for remote use, and no longer
disables TLS certificate verification. `tests/no_implicit_remote.rs` checks
the local-only default. These components are not yet connected to the GUI.
