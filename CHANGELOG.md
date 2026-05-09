# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-09

Initial release.

### Added
- Tree-style directory listing with box-drawing characters.
- Comments loaded from a `.treedoc.yaml` sidecar, aligned per row.
- `.gitignore` and dotfile filtering, configurable via `--no-ignore` / `-a`.
- `-L/--depth` to limit recursion depth.
- Colored TTY output with `NO_COLOR` / `--no-color` honoured.
- `--format console|plain|markdown` for terminal, plain-text, and Markdown
  fenced output.
- `treedoc init`, `treedoc set`, and `treedoc edit` subcommands for
  managing the sidecar.

[Unreleased]: https://github.com/kukv/treedoc/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kukv/treedoc/releases/tag/v0.1.0
