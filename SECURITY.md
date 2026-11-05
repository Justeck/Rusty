# Security policy

Security fixes target the latest published Rusty release. Compatibility is limited to the game and Horizon revisions documented for that release.

## Report a vulnerability privately

Use [GitHub's private vulnerability reporting](https://github.com/Justeck/Rusty/security/advisories/new) if available for this repository. Include the affected version, impact, and minimal reproduction. Do not post working exploits, tokens, account details, or other sensitive information in public issues.

If private reporting is unavailable, open an issue asking the maintainer for a private reporting channel, without including sensitive vulnerability details. Response and resolution times are not guaranteed.

## Native-source installation

Horizon compiles native mods into the client. Review the source and the launcher's native-source prompt before installing. Obtain packages from [official releases](https://github.com/Justeck/Rusty/releases) and compare a manually downloaded package with the supplied `SHA256SUMS`.

Profiles contain local settings. Do not include credentials or secrets in shared profiles or report attachments.
