# Reborn Addon Manager

An independent addon manager for Azeroth Survivors Reborn, based on
[Rezondes' SurvivorsForge](https://github.com/Rezondes/SurvivorsForge).

The new main branch starts from upstream v0.3.2. Selected local improvements from
v0.8.0 are being integrated for Reborn. This is an independent version maintained
in this fork. We do not plan to submit its changes back to upstream. The original
repository is consulted manually for selected improvements and fixes.

**Preview build.** A production catalog address and approved starter
addons are still pending coordination with the Reborn team.

- Select the game's Interface/AddOns folder. Paths are discovered locally and are
  never compiled from the developer's personal installation.
- Launch AzerothSurvivorsLauncher.exe or AzerothSurvivorsGame.exe separately.
- English by default, with Russian and German UI options.
- Install addon ZIPs or GitHub repositories; use the independent Reborn catalog
  once its hosting is configured.
- Local security scanning, SHA-256 checks, transactional addon updates and
  per-character addon switches. Close the game before changing its addon settings.
- SavedVariables backups stay locally in the game folder, under
  .reborn-addon-manager/backups. Account configuration is excluded.

No Rezondes API, website, catalog or original game-update service is used at
runtime. Addon files are downloaded directly from their authors' GitHub releases.
The catalog begins empty until inclusion and Reborn compatibility are confirmed.
There are no platform accounts, comments, cloud profiles or telemetry.

## Build and test

Rust stable, Slint's native platform requirements and Node 24 for catalog tests:

~~~powershell
cargo test --locked --workspace
node --test catalog/generate.test.mjs
$env:REBORN_CATALOG_URL = 'https://<agreed-subdomain>/v1/registry'
cargo build --release --locked -p survivorsforge
~~~

Windows output: target/release/RebornAddonManager.exe. Configuration is written
on the recipient's computer under the application's own data directory.
REBORN_ADDON_MANAGER_DATA_DIR can override that directory for isolated testing.
Do not distribute personal config/state files together with the executable.

See [catalog setup](catalog/README.md) and [migration status](docs/REBORN_MIGRATION.md).
Cloudflare deployment instructions follow the [official Direct Upload guide](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/).

## Credits and license

Original project and code: Rezondes and the SurvivorsForge contributors.
Reborn adaptation: AlexEnotov, 2026-11-09.
Original base: v0.3.2 (91252de1ab99d7a3fa188e552575687161b1511b).
Ported modules retain their original notices and are derived from v0.8.0.

Licensed under AGPL-3.0-or-later; see [LICENSE](LICENSE). Distributed builds must
have their corresponding source available under this license. Third-party assets
retain their included license notices. This project is not the official
SurvivorsForge client.
