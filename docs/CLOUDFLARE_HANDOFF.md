# Reborn team: catalog hosting handoff

The launcher and generator do not need a SurvivorsForge backend. Cloudflare serves
static metadata; addon ZIPs stay on GitHub. The reviewed manifest repository is
https://github.com/AlexEnotov/reborn-addons.

## Information still needed

- The Reborn-owned catalog hostname (for example, an agreed subdomain).
- Cloudflare account ID and Pages project name.
- The GitHub username of the teammate managing hosting/secrets.
- Starter addon author consent and confirmation that the addons run in Reborn.

## Cloudflare setup

1. Create a Cloudflare Pages project using Direct Upload. Record its project name.
2. Add the agreed custom domain in the project's custom-domain settings and complete
   the DNS setup in the Reborn team's account.
3. Create an API token with Account / Cloudflare Pages / Edit permissions for the
   intended Cloudflare account. Keep domain/DNS changes as a separate manual step.
4. In AlexEnotov/reborn-addons, add repository Actions secrets:
   CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID. Add the token directly to Secrets;
   do not send it through chat, paste it into manifests or commit it.
5. Add repository variable CLOUDFLARE_PROJECT_NAME with the exact Pages project name.
   SCANNER_REF must contain the reviewed full commit SHA in the launcher repository.
6. Run the Reborn catalog workflow manually. It builds the pinned local scanner,
   reads reviewed manifests, generates JSON and deploys public/ using Wrangler.
   Subsequent changes run after reviewed pushes and on the hourly schedule.

Repository workflows can use repository secrets. Agree on this access with the
team before adding the token. PR runs do not receive deployment credentials.
If generation or scanning fails, deployment is not run and the existing catalog
remains served. No addon ZIPs are uploaded to Cloudflare.

## Validate the deployment

The extensionless /v1/registry response must have:

- Content-Type: application/json (optionally with charset=utf-8).
- Cache-Control: public, max-age=300.
- An ETag; a repeated request with If-None-Match must return 304.

Example using the agreed hostname:

~~~sh
curl -I https://YOUR-HOST/v1/registry
curl -i -H 'If-None-Match: COPY-THE-ETAG-HERE' https://YOUR-HOST/v1/registry
~~~

After these checks, set REBORN_CATALOG_URL in the launcher repository variables to
https://YOUR-HOST/v1/registry and build a new Windows package. This is a build-time
setting; it does not place a developer's local game path in the executable.

Before announcing a player release, test a fresh install, offline cached catalog,
one successful addon installation, refusal of a wrong hash, and discovery of a
new test-addon release through the hourly workflow. Test starter addons in Reborn
with the game developers; a clean security scan alone does not establish gameplay
compatibility.

Official Cloudflare guide:
https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/
