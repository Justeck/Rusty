# Contributing to Rusty

Bug reports and focused improvements are welcome. For substantial behavior or compatibility changes, open an issue describing the problem and proposed result before beginning a large implementation.

## Report a problem

Use the [bug report form](https://github.com/Justeck/Rusty/issues/new?template=bug_report.yml). Include Rusty version, game/Horizon versions, operating system, enabled native mods, reproduction steps, expected behavior, and observed behavior. A small local-world reproduction is useful.

Remove tokens, account information, private server addresses, coordinates, and personal paths from attachments. Security issues should follow [SECURITY.md](SECURITY.md).

## Submit a change

Keep a pull request focused on one problem. Describe the resulting behavior and relevant verification results. Follow existing Rust style and the [development guide](docs/DEVELOPMENT.md). Native hooks or rendering changes need checks against pinned Horizon source and an interactive check where applicable; root tests alone do not establish in-game behavior.

Update documentation when controls, defaults, limits, or installation change. Preserve required copyright, attribution, and component notices. Do not submit game assets, resource-pack artwork, accounts, saves, or private logs.

Contributions are made under the project's applicable [license and notices](LEGAL.md).
