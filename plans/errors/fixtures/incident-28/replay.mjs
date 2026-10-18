import {readFile,writeFile} from 'node:fs/promises';
import {createServer} from 'node:http';
import {spawn} from 'node:child_process';
const host=JSON.parse(await readFile('/tmp/incident28-host.json','utf8'));
const manifest=JSON.parse(await readFile('fixtures/providers/manifest.json','utf8'));
const bin='src-tauri/binaries/scrape-worker-aarch64-apple-darwin';
const results=[];
for(const workflow of host) for(const child of workflow.children){
 const source=structuredClone(child.worker), key=source.name.toLowerCase();
 const meta=key==='cloudflare'?{file:'/tmp/incident28-cloudflare.json',capture_url:'https://boards-api.greenhouse.io/v1/boards/cloudflare/jobs?content=true'}:manifest[key];
 const bytes=await readFile(meta.file), listing=new URL(meta.capture_url);
 const requests=[];
 const server=createServer((req,res)=>{
  requests.push(req.url);
  if(req.url.endsWith('/robots.txt')){res.end('User-agent: *\nAllow: /');return;}
  const path=new URL(req.url,'http://fixture.invalid').pathname;
  if(path===listing.pathname||path===`/${listing.host}${listing.pathname}`){
   res.setHeader('Content-Type',meta.file.endsWith('html')?'text/html':'application/json');res.end(bytes);
  }else {res.statusCode=404;res.end('Uncaptured detail/asset; unavailable in diagnostic replay');}
 });
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const origin=`http://127.0.0.1:${server.address().port}`;
 if(source.config.mode==='script')source.config.fixture_path=meta.file;
 const r=workflow.filters.matching;
 const variants={unrestricted:{},template:workflow.filters,keywords:{matching:{keywords:r.keywords}},role:{matching:{role_types:r.role_types}},workplace:{matching:{workplaces:r.workplaces}},geography:{matching:{countries:r.countries,regions:r.regions,cities:r.cities}}};
 for(const [variant,filters] of Object.entries(variants)){
  requests.length=0;
  const cfg={version:7,sources:[source],filters,filters_by_source:{},run_started_at:'2026-10-18T12:00:00Z',max_jobs_per_run:20,limits:{concurrency:1,per_host_delay_ms:0,request_timeout_s:3}};
  const output=await new Promise((resolve,reject)=>{
   const child=spawn(bin,['--stdin','--replay-origin',origin],{stdio:['pipe','pipe','pipe']});let stdout='',stderr='';
   const timer=setTimeout(()=>{child.kill('SIGKILL');reject(new Error('diagnostic worker timed out '+key))},60000);
   child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);child.on('error',reject);
   child.on('close',code=>{clearTimeout(timer);code===0?resolve(stdout):reject(new Error(`${key}: ${code}: ${stderr}`))});child.stdin.end(JSON.stringify(cfg)+'\n');
  });
  const events=output.trim().split('\n').filter(Boolean).map(x=>JSON.parse(x));
  await writeFile(`/tmp/incident28-${key}-${variant}.ndjson`,output);
  const jobs=events.filter(e=>e.type==='job'), counts=field=>jobs.reduce((out,j)=>(out[j[field]??'missing']=(out[j[field]??'missing']??0)+1,out),{});
  const result={key,variant,jobs:jobs.length,roles:counts('role_type'),workplaces:counts('workplace_type'),completed:events.at(-1)?.stats?.sources??[],errors:events.filter(e=>e.type==='error'),warnings:events.filter(e=>e.type==='warning').length,requests:requests.length};
  results.push(result);console.log(JSON.stringify(result));
 }
 await new Promise(r=>server.close(r));
}
await writeFile('/tmp/incident28-replay.json',JSON.stringify(results,null,2));
