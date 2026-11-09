// Independent Reborn catalog. Addon ZIPs stay on GitHub; only metadata is published.
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';

// Deliberately limited TOML subset: flat keys, JSON-compatible strings/string arrays.
// Unsupported TOML is rejected instead of being interpreted ambiguously.
export function parseManifest(text) {
  const result = {};
  for (const [index, raw] of text.split(/\r?\n/).entries()) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const match = /^([a-z_]+)\s*=\s*(.+)$/.exec(line);
    if (!match || Object.hasOwn(result, match[1])) throw Error(`Invalid or duplicate manifest key at line ${index+1}`);
    const allowed = ['repo','name','author','description','category','folders','asset','icon','verified_tag','verified_sha256'];
    if (!allowed.includes(match[1])) throw Error(`Unsupported key: ${match[1]}`);
    const value = JSON.parse(match[2]);
    if (match[1] === 'folders' ? !Array.isArray(value) || !value.length || value.some(v=>typeof v !== 'string') : typeof value !== 'string') throw Error(`Invalid ${match[1]}`);
    result[match[1]] = value;
  }
  if (!/^[\w.-]+\/[\w.-]+$/.test(result.repo ?? '') || !result.name || !result.category || !result.asset || !result.folders?.length) throw Error('Incomplete addon manifest');
  if (result.folders.some(f=>!f || /[<>:"/\\|?*\x00-\x1f]/.test(f) || /[. ]$/.test(f) || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(f))) throw Error('Unsafe addon folder');
  if (Boolean(result.verified_tag) !== Boolean(result.verified_sha256) || (result.verified_sha256 && !/^[a-f\d]{64}$/i.test(result.verified_sha256))) throw Error('Verification requires both tag and SHA-256');
  result.icon = (result.icon ?? '').trim();
  if (result.icon && (result.icon.startsWith('/') || /[\\:\x00-\x1f]/.test(result.icon)
      || result.icon.split('/').some(p=>!p || p==='.' || p==='..')
      || !/\.(png|jpe?g|webp|tga|bmp|ico)$/i.test(result.icon))) throw Error('icon must be a relative image path in the addon repository, or an empty string');
  return result;
}

export function iconUrl(repo, branch, icon) {
  if (!icon) return null;
  return `https://raw.githubusercontent.com/${repo.split('/').map(encodeURIComponent).join('/')}/${encodeURIComponent(branch || 'main')}/${icon.split('/').map(encodeURIComponent).join('/')}`;
}

export function allowedUrl(raw, api = false) {
  const url = new URL(raw);
  const hosts = api ? ['api.github.com'] : ['github.com','release-assets.githubusercontent.com','objects.githubusercontent.com','github-releases.githubusercontent.com'];
  if (url.protocol !== 'https:' || url.username || url.password || !hosts.includes(url.hostname)) throw Error('Disallowed catalog network destination');
  return url;
}

async function request(raw, api = false) {
  let url = allowedUrl(raw, api);
  for (let hop=0;hop<8;hop++) {
    const headers = {'User-Agent':'RebornAddonCatalog/0.1.0'};
    if (api) {
      headers.Accept = 'application/vnd.github+json';
      if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
    }
    const response = await fetch(url,{headers,redirect:'manual',signal:AbortSignal.timeout(120000)});
    if (response.status >= 300 && response.status < 400) {
      url = allowedUrl(new URL(response.headers.get('location'),url).href,api);
      await response.body?.cancel();
      continue;
    }
    if (!response.ok) throw Error(`GitHub request failed: ${response.status}`);
    return response;
  }
  throw Error('Too many redirects');
}

export function matches(pattern, name) {
  return new RegExp('^'+pattern.split('*').map(p=>p.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')).join('.*')+'$','i').test(name);
}

export async function generate(manifestDir, outputDir, scanner, transport = request) {
  const files = (await fs.readdir(manifestDir)).filter(f=>f.endsWith('.toml')).sort();
  const previous = await fs.readFile(path.join(outputDir,'v1/registry'),'utf8').then(JSON.parse).catch(()=>({addons:[]}));
  const addons = [], claimed = new Set();
  for (const file of files) {
    const id = path.basename(file,'.toml');
    if (!/^[a-z0-9][a-z0-9-]*$/.test(id)) throw Error(`Invalid addon id: ${id}`);
    const manifest = parseManifest(await fs.readFile(path.join(manifestDir,file),'utf8'));
    for (const folder of manifest.folders) {
      if (claimed.has(folder.toLowerCase())) throw Error(`Folder used twice: ${folder}`);
      claimed.add(folder.toLowerCase());
    }
    const repo = await (await transport(`https://api.github.com/repos/${manifest.repo}`,true)).json();
    const release = await (await transport(`https://api.github.com/repos/${manifest.repo}/releases/latest`,true)).json();
    const assets = release.assets.filter(a=>matches(manifest.asset,a.name));
    if (assets.length !== 1) throw Error(`${id}: expected exactly one matching release asset`);
    const asset = assets[0];
    allowedUrl(asset.browser_download_url);
    if (!Number.isSafeInteger(asset.size) || asset.size <= 0 || asset.size > 512*1024*1024) throw Error(`${id}: invalid asset size`);
    const response = await transport(asset.browser_download_url);
    const chunks = []; let size = 0;
    for await (const chunk of response.body) {
      size += chunk.length;
      if (size > 512*1024*1024) throw Error(`${id}: download too large`);
      chunks.push(Buffer.from(chunk));
    }
    if (size !== asset.size) throw Error(`${id}: incomplete download`);
    const bytes = Buffer.concat(chunks), sha256 = createHash('sha256').update(bytes).digest('hex');
    const temp = await fs.mkdtemp(path.join(os.tmpdir(),'reborn-scan-'));
    let report;
    try {
      const zip = path.join(temp,'addon.zip');
      await fs.writeFile(zip,bytes);
      const scan = spawnSync(scanner,[zip,'--folders',manifest.folders.join(',')],{encoding:'utf8',timeout:120000,maxBuffer:8*1024*1024,windowsHide:true});
      if (scan.error || ![0,1].includes(scan.status)) throw Error(`${id}: scanner failed`);
      report = JSON.parse(scan.stdout);
      if (!Array.isArray(report.findings)) throw Error(`${id}: invalid scanner output`);
      report.flagged = scan.status === 1;
    } finally { await fs.rm(temp,{recursive:true,force:true}); }
    const pinned = manifest.verified_tag === release.tag_name && manifest.verified_sha256?.toLowerCase() === sha256;
    const trust = report.flagged ? 'flagged' : pinned ? 'verified' : 'scanned';
    const old = previous.addons.find(a=>a.id===id);
    // Keep the counter snapshot for an unchanged artifact so hourly runs are idempotent.
    const downloads = old?.latest?.sha256 === sha256 ? old.downloads : asset.download_count;
    addons.push({added_at:old?.added_at || Math.floor(Date.now()/1000),id,name:manifest.name,author:manifest.author ?? repo.owner.login,repo:manifest.repo,description:manifest.description ?? repo.description ?? '',category:manifest.category,folders:manifest.folders,icon_url:iconUrl(manifest.repo,repo.default_branch,manifest.icon),source:{type:'release',asset:manifest.asset},trust,downloads,latest:{tag:release.tag_name,asset_url:asset.browser_download_url,sha256,size,trust,findings:report.findings,published_at:release.published_at}});
  }
  const v1 = path.join(outputDir,'v1'); await fs.mkdir(v1,{recursive:true});
  for (const [name,value] of Object.entries({registry:{schema:1,addons},categories:[...new Set(addons.map(a=>a.category))].sort(),'game-versions':['11200','30300']})) {
    await fs.writeFile(path.join(v1,name),JSON.stringify(value,null,2)+'\n');
  }
  await fs.writeFile(path.join(outputDir,'_headers'),'/v1/*\n  Content-Type: application/json; charset=utf-8\n  Cache-Control: public, max-age=300\n');
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  await generate(process.argv[2] ?? 'addons',process.argv[3] ?? 'public',process.env.FORGE_SCAN ?? 'forge-scan');
}
