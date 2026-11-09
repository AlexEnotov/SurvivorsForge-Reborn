# Reborn Addon Manager migration

Based on Rezondes/SurvivorsForge tag v0.3.2, commit
`91252de1ab99d7a3fa188e552575687161b1511b`.
Selected local modules are taken from upstream v0.8.0, commit
`7f769e0297c31d76f79f9cf490f8e2865e422a2d`.
Modified for Reborn on 2026-11-09.
This is an independent Reborn version. Its development happens in this fork;
changes stay in this fork and we do not plan to submit them upstream. Original
source is consulted manually when selecting improvements and fixes.

Preserve the AGPL-3.0-or-later license and original copyright notices.
Original project: https://github.com/Rezondes/SurvivorsForge

## Functional foundation

- Imported: shared archive validation, folder validation, TOC parser, transactional
  installation and removal helpers, local forge-scan and Slint toast library.
- Adapted: Reborn Interface/AddOns, two EXE names, per-character addon switches,
  11200/30300 compatibility, English default and Russian/German translations.
- Network isolation: no original catalog, backend, telemetry or game updater;
  own User-Agent, manual validated redirects, GitHub-only addon downloads.
- Independent catalog generator: reviewed manifests, GitHub releases, SHA-256,
  scanner subprocess, stable JSON and Cloudflare headers.
- Integrated: Reborn SavedVariables backup and restore, removal undo,
  periodic checks, window placement/page persistence, minimize/restore on direct
  game launch, duplicate game-launch checks, required dependencies, download retry,
  hash-bound scan consent and update-first ordering.
- Verified locally: transactional installs/rollback, archive traversal rejection,
  hash mismatch refusal, local scanner, saved settings snapshots, addon switches,
  blocked redirects before contacting author services, and generator output
  accepted by the actual launcher parser.
- Redesigned desktop UI: sidebar navigation, persistent launch actions, searchable
  filtered cards with explicit addon switches, full local details/error dialogs,
  grouped scrollable settings, EN/RU/DE layout checks and keyboard shortcuts.
- Optional grid/tile views, pack export and toast presentation are not exposed
  in this edition. The imported pack/toast libraries remain available.
- Pending team inputs: Cloudflare project/domain/token and approved starter addons.
- Deliberately omitted: original backend, accounts, comments, issues, cloud
  profiles/sync, telemetry and original game downloads.

The previous published Reborn binary has been retired. Its exact corresponding
source remains attached to the historical release as
`SurvivorsForge-Reborn-0.7.2-source.zip`; the historical tag is retained solely for
source provenance. Active development starts from v0.3.2 on main.

See INTERFACE.md for the implemented interface and offline visual verification.

## Upstream audit notes

- `d618828`: shared archive/TOC code imported from v0.8.0.
- `9cb5782`, `1924291`, `b61a0e2`, `b3ab557`, `7d3274a`, `537d64a`: scanner
  and subsequent parser/size/panic fixes included in imported scanner modules.
- `e402a6f`, `39c0407`: Windows folder identity and short-name checks included.
- `8cf5466`: held-open folder rollback behavior included in transactional installer.
- `e299bff`, `ad42fd0`: validated redirects and one retry implemented in direct GitHub client.
- `a7184bd`, `9852c65`: hash checks/local scan adapted for static catalogs and manual ZIPs.
- `61b30eb`: required dependencies adapted from local TOC metadata and own catalog.
- `3c6443d`: settings backups adapted to WTF, rather than benilla-config.
- `cc18ca1`, `df21d28`, `f103f2b`, `f25a804`, `8bef596`: periodic checks,
  update ordering and window/game lifecycle behavior adapted for Reborn.
- Original game updater/self-replacement, backend routing/auth/comments/issues,
  telemetry and cloud profiles are deliberately not used.

The catalog generator is maintained in this repository under catalog/. Reviewed
manifests and deployment workflow live in AlexEnotov/reborn-addons; SCANNER_REF
pins both generator and scanner to a reviewed commit here.
