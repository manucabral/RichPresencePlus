# Security

Please do not report a vulnerability in a public issue.

Use GitHub's private reporting: the **Report a vulnerability** button on the
[Security tab](https://github.com/manucabral/RichPresencePlus/security/advisories/new)
of this repository. Include the version, the steps to reproduce it and what an
attacker gains. A presence in the catalog that misbehaves counts too.

Only the latest release receives fixes.

These are especially welcome: a presence reading beyond its manifest, a web
page reaching the local bridge, a catalog item installing without a valid
signature, and an update accepted without one.

## How the application protects itself

- The extension and the application talk on `127.0.0.1` only. Both sides prove
  they know the pairing token with HMAC-SHA256, and every later message is
  sealed with AES-256-GCM, with one key per direction. A message out of order
  or repeated is refused (`crates/rpp-bridge/src/secure.rs`).
- Each presence runs in its own QuickJS sandbox, with 16 MB of memory, 2 seconds
  per read and no access beyond its manifest (`crates/rpp-plugins`).
- The catalog is signed with Ed25519 (`crates/rpp-catalog`), and updates are
  checked with minisign before they install.
