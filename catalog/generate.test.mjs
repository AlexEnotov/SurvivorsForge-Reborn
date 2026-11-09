import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {parseManifest, matches, allowedUrl, generate, iconUrl} from './generate.mjs';

const valid = 'repo = "example/addon"\nname = "Test Addon"\ncategory = "Combat"\nfolders = ["TestAddon"]\nasset = "TestAddon-*.zip"\n';

test('validates manifest paths, types, duplicates and verification pins',()=>{
  assert.equal(parseManifest(valid).repo,'example/addon');
  for (const bad of [valid+'name = "Other"',valid.replace('"TestAddon"]','"../Escape"]'),valid.replace('"TestAddon"]','"NUL"]'),valid+'verified_tag = "v1"',valid+'unknown = "x"']) assert.throws(()=>parseManifest(bad));
  assert.equal(matches('Addon-*.zip','Addon-v1.zip'),true);
  assert.equal(matches('Addon-*.zip','Addon-v1Xzip'),false);
});

test('rejects backend hosts and credentials before fetching',()=>{
  for (const url of ['https://forgeapi.rezondes.net/v1/registry','https://wow.aethus.pl/game.zip','https://github.com.evil.org/a.zip','http://github.com/a.zip','https://token@github.com/a.zip']) assert.throws(()=>allowedUrl(url));
  assert.equal(allowedUrl('https://github.com/example/addon/releases/download/v1/addon.zip').hostname,'github.com');
});

test('optional icons use a relative repository path, not a URL or parent traversal',()=>{
  assert.equal(parseManifest(valid).icon,'');
  assert.equal(parseManifest(valid+'icon = ""').icon,'');
  const manifest=parseManifest(valid+'icon = "assets/addon logo.png"');
  assert.equal(iconUrl(manifest.repo,'main',manifest.icon),'https://raw.githubusercontent.com/example/addon/main/assets/addon%20logo.png');
  assert.equal(iconUrl(manifest.repo,'main',''),null);
  for (const value of ['../icon.png','/icon.png','https://example.org/icon.png','assets/../icon.png','icon.svg','none']) {
    assert.throws(()=>parseManifest(valid+'icon = '+JSON.stringify(value)));
  }
});

test('empty independent catalog is deterministic and makes no network requests',async()=>{
  const temp=await fs.mkdtemp(path.join(os.tmpdir(),'reborn-catalog-test-'));
  try {
    const manifests=path.join(temp,'addons'), output=path.join(temp,'public');
    await fs.mkdir(manifests);
    const noNetwork=()=>{throw Error('Unexpected network request')};
    await generate(manifests,output,'unused',noNetwork);
    const first=await fs.readFile(path.join(output,'v1/registry'),'utf8');
    assert.deepEqual(JSON.parse(first),{schema:1,addons:[]});
    await generate(manifests,output,'unused',noNetwork);
    assert.equal(await fs.readFile(path.join(output,'v1/registry'),'utf8'),first);
    assert.match(await fs.readFile(path.join(output,'_headers'),'utf8'),/max-age=300/);
  } finally {await fs.rm(temp,{recursive:true,force:true});}
});

function fixtureZip(text) {
  const name=Buffer.from('TestAddon/TestAddon.toc'), data=Buffer.from(text);
  let crc=0xffffffff;
  for(const byte of data) { crc ^= byte; for(let i=0;i<8;i++) crc=(crc>>>1)^((crc&1)?0xedb88320:0); }
  crc=(crc^0xffffffff)>>>0;
  const local=Buffer.alloc(30), central=Buffer.alloc(46), end=Buffer.alloc(22);
  local.writeUInt32LE(0x04034b50); local.writeUInt16LE(20,4); local.writeUInt32LE(crc,14); local.writeUInt32LE(data.length,18); local.writeUInt32LE(data.length,22); local.writeUInt16LE(name.length,26);
  central.writeUInt32LE(0x02014b50); central.writeUInt16LE(20,4); central.writeUInt16LE(20,6); central.writeUInt32LE(crc,16); central.writeUInt32LE(data.length,20); central.writeUInt32LE(data.length,24); central.writeUInt16LE(name.length,28);
  end.writeUInt32LE(0x06054b50); end.writeUInt16LE(1,8); end.writeUInt16LE(1,10); end.writeUInt32LE(central.length+name.length,12); end.writeUInt32LE(local.length+name.length+data.length,16);
  return Buffer.concat([local,name,data,central,name,end]);
}

test('generates a scanned artifact with stable SHA-256 using the real local scanner', {skip:!process.env.FORGE_SCAN}, async()=>{
  const {createHash}=await import('node:crypto');
  const temp=await fs.mkdtemp(path.join(os.tmpdir(),'reborn-catalog-integration-'));
  try {
    const manifests=path.join(temp,'addons'), output=path.join(temp,'public');
    await fs.mkdir(manifests); await fs.writeFile(path.join(manifests,'test-addon.toml'),valid+'icon = "assets/icon.png"');
    const zip=fixtureZip('## Interface: 30300\n## Title: Test Addon\n');
    let downloads=7;
    const transport=async(url)=> {
      if(url.endsWith('/releases/latest')) return {json:async()=>({tag_name:'v1',published_at:'2026-11-09T00:00:00Z',assets:[{name:'TestAddon-v1.zip',browser_download_url:'https://github.com/example/addon/releases/download/v1/TestAddon-v1.zip',size:zip.length,download_count:downloads}]})};
      if(url==='https://api.github.com/repos/example/addon') return {json:async()=>({owner:{login:'example'},description:'A fixture'})};
      if(url==='https://github.com/example/addon/releases/download/v1/TestAddon-v1.zip') return {body:[zip]};
      throw Error('Unexpected network destination');
    };
    await generate(manifests,output,process.env.FORGE_SCAN,transport);
    const first=await fs.readFile(path.join(output,'v1/registry'),'utf8');
    const addon=JSON.parse(first).addons[0];
    assert.equal(addon.icon_url,'https://raw.githubusercontent.com/example/addon/main/assets/icon.png');
    if (process.env.REBORN_MANAGER_CHECK) {
      const {spawnSync}=await import('node:child_process');
      const check=spawnSync(process.env.REBORN_MANAGER_CHECK,['--check-catalog',path.join(output,'v1/registry')],{encoding:'utf8',windowsHide:true});
      assert.equal(check.status,0,check.stderr);
      assert.match(check.stdout,/Valid Reborn catalog: 1 addons/);
    }
    assert.equal(addon.trust,'scanned'); assert.equal(addon.latest.tag,'v1');
    assert.equal(addon.latest.sha256,createHash('sha256').update(zip).digest('hex'));
    assert.equal(addon.latest.size,zip.length);
    downloads=9;
    await generate(manifests,output,process.env.FORGE_SCAN,transport);
    assert.equal(await fs.readFile(path.join(output,'v1/registry'),'utf8'),first);
  } finally {await fs.rm(temp,{recursive:true,force:true});}
});
