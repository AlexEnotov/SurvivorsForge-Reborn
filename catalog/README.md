# Reborn addon catalog

The generator source lives here. Reviewed addon manifests and the deployment
workflow live in [reborn-addons](https://github.com/AlexEnotov/reborn-addons).
That workflow checks out this repository at its pinned SCANNER_REF commit.
No addon is included until its author agrees to inclusion and it has been tested in Reborn.
ZIP downloads remain on the authors' GitHub releases. No Rezondes service is used.

Each addon is submitted as `addons/<id>.toml` by pull request:

```toml
repo = "example/MyAddon"
name = "My Addon"
category = "Combat"
folders = ["MyAddon"]
asset = "MyAddon-*.zip"
icon = ""
```

Optional fields: `author`, `description`, `verified_tag`, `verified_sha256`.
Verification requires both an exact release tag and the hash of the reviewed ZIP.
The supported TOML subset is flat keys with double-quoted strings and string arrays;
comments must occupy a separate line. Unsupported syntax is rejected.

Contributors can submit PRs. Maintainers review author consent, folder ownership,
Reborn compatibility and release asset selection before merging. A GitHub account
is needed to submit; downloading addons does not require one.

With Node 24 and the separately built `forge-scan` executable:

```
node --test generate.test.mjs
node generate.mjs addons public
```

Set `FORGE_SCAN` to an absolute path to the scanner and, optionally, `GITHUB_TOKEN`
to a token with public repository read access. Tokens are sent only to api.github.com.
The generator follows only vetted GitHub asset redirects, checks download size,
calculates SHA-256 and invokes the scanner as a subprocess. Scanner failure aborts
generation; the existing hosted catalog must remain published.

Output: `public/v1/registry`, `public/v1/categories`, `public/v1/game-versions`,
and `public/_headers`. Install the complete output as one atomic deployment.
`downloads` is a snapshot taken when an artifact changes; an unchanged release
produces identical output even when GitHub's live download counter increases.

Trust means `scanned` (no findings), `flagged` (requires player confirmation), or
`verified` (a clean artifact with a maintainer-pinned tag and SHA-256).
A clean scan is not a guarantee of safety. Findings always take priority over pins.

## Cloudflare handoff

The Reborn team supplies the project name, account ID, subdomain and a scoped
deployment API token. Add the token directly to GitHub Secrets as
`CLOUDFLARE_API_TOKEN` and the account ID as `CLOUDFLARE_ACCOUNT_ID`.
Repository workflows can use these secrets, so their access must be agreed with
the team. Do not send tokens in chat or commit them.

The scheduled workflow must run hourly and after reviewed manifest changes.
PR workflows validate only and must never receive deployment secrets.
Deployment remains pending until the team supplies the hosting details.
After deployment check Content-Type, max-age=300 and a conditional request
returning 304, then build the launcher with
`REBORN_CATALOG_URL=https://<agreed-subdomain>/v1/registry`.

Acceptance checks still requiring external coordination: approved starter addons
running in Reborn; catalog discovery from a fresh install; a test addon update
within an hour; live Cloudflare headers and conditional requests.

## Addon icons

Each manifest may set `icon = "assets/icon.png"`, a path inside that addon’s
repository on its default branch. Omit the field or use `icon = ""` for no icon.
Do not use `none`, absolute URLs, parent traversal or Windows backslashes.
Supported images: PNG, JPEG, WebP, TGA, BMP and ICO; PNG is recommended.
The generator emits icon_url; it never mirrors image files to Cloudflare.
The launcher caches bounded images from the same GitHub repository, falls back
to local IconTexture/common addon artwork, and displays a placeholder if no
image is available. Missing/broken icons do not prevent installing an addon.
