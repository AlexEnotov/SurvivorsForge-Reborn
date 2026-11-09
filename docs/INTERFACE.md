# Desktop interface

Updated 2026-11-09. Native Slint interface; dark blue theme, Segoe UI.

## Structure

- Persistent sidebar: Installed, Catalog and Settings. Check & play runs Reborn's
  update/error checks before starting the game; Play directly skips these checks.
  Both actions remain visible at the bottom; the app's language selector lives in Settings
  and on the first-run screen.
- Installed: search, All/Enabled/Disabled/Updates filters, cards with source,
  version, status, explicit switches and separate update/remove actions.
- Mixed character settings are shown as Mixed, with a distinct amber switch.
- Catalog: shows a useful empty state while hosting is not configured; ZIP and
  GitHub installation remain available. No external catalog is requested for UI previews.
- Settings: scrollable cards for game installation, language, backups and credits.
- Addon details show an icon, author, version, description and clearly labelled
  folders in a compact dialog. Installed icons are read locally; optional catalog
  icons use a repository-relative `icon` manifest field. Missing or invalid images
  use a letter placeholder. Activity text can be copied from its separate dialog.
- Support for this Reborn version belongs to AlexEnotov. The persistent Report a
  problem link opens this repository's Issues. About separates this support and
  source information from the original SurvivorsForge attribution to Rezondes.
- A fantasy blue book with a gold R is used in the sidebar, window and Windows
  executable. Earlier icon variants remain in assets/branding. The sidebar mark
  is 72px beside the name, with both text lines vertically centred against it.
  The window caption does not repeat the sidebar's app name, and the page header
  uses a compact top inset below the window controls.

## Shared components and tokens

Theme colors and typography live in ui/theme.slint. Surface and line colors are
neutral; primary actions, selected navigation and focus use blue. Green denotes
success, amber denotes compatibility warnings/mixed state, red denotes errors.
Decorative colors are separate from semantic states.

ui/workspace.slint contains NavItem, FilterChip, SettingCard, EmptyState and
AddonSwitch. ui/components.slint contains shared actions, text fields, cards,
scrolling, dialogs and the native window frame. New controls expose accessible
labels/default actions; switches expose checked state. Keyboard focus scopes
inside controls have zero geometry to avoid intercepting pointer clicks.

Window minimum: 980 x 660 logical pixels. Lists and settings scroll, descriptions
wrap or open in a detail dialog, and language labels retain their native names.
English remains the default; Russian and German use bundled translations.

## Keyboard

- Ctrl+1 / Ctrl+2 / Ctrl+3: Installed / Catalog / Settings.
- Ctrl+F: focus search, returning to Installed from Settings.
- F5: check updates. Ctrl+U: update all.
- Escape: close the current dialog or clear search.
- Tab, Space and Enter: reach and activate controls.

## Offline verification

~~~powershell
cargo test --locked --bin RebornAddonManager capture_ui -- --ignored
~~~

Visual fixtures are written to .qa/ui/ and contain sample data only. The capture
checks real pointer clicks on an addon switch, both game launch actions and the
support link, sidebar navigation, keyboard
navigation/refresh, settings scrolling and EN/RU/DE layouts. It does not load user
configuration, launch the game, modify addons or contact any server. GitHub CI
also saves the resulting screenshots as an artifact for review.
