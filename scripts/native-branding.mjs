import { runNativeDev } from "./tauri-dev-test.mjs";
import { createServer } from "node:http";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
const directory=await mkdtemp(join(tmpdir(),"scrape-native-smoke-"));
const original=await readFile("crates/scrape-worker/tests/fixtures/cloudflare-logo.jpg");
let online=true,downloads=0;
const server=createServer((request,response)=>{
  response.setHeader("Access-Control-Allow-Origin","*");
  if(request.url==="/offline"){online=false;response.end("offline fixture");return;}
  if(request.url==="/logo.png"){downloads++;response.statusCode=online?200:404;response.end(online?original:"Authored unavailable image");return;}
  if(request.url==="/robots.txt"){response.end("User-agent: *\nAllow: /");return;}
  response.end(`<script type="application/ld+json">${JSON.stringify([42,43].map(id=>({"@type":"JobPosting",identifier:String(id),title:`Native branding engineer ${id}`,url:`/jobs/${id}`,hiringOrganization:{name:"Cloudflare",logo:"/logo.png"}})))}</script>`);
});
await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));const origin=`http://127.0.0.1:${server.address().port}`;
await writeFile(join(directory,"smoke.js"),`(async()=>{
 const invoke=window.__TAURI_INTERNALS__.invoke,assert=(value,message)=>{if(!value)throw new Error(message);};
 const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
 const wait=async predicate=>{const until=Date.now()+12000;while(Date.now()<until){const value=predicate();if(value)return value;await pause(50);}throw new Error("Native logo UI did not settle");};
 let expected=sessionStorage.getItem("branding-asset"),reference=sessionStorage.getItem("branding-reference");
 function validateAsset(asset){
  assert(asset?.startsWith("data:image/webp;base64,"),"New cache write did not use WebP");
  const bytes=atob(asset.split(",")[1]);assert(bytes.startsWith("RIFF")&&bytes.slice(8,12)==="WEBP","Cache MIME does not match WebP content");return bytes.length;
 }
 async function visibleLogo(size){
   const img=await wait(()=>[...document.querySelectorAll('img[alt="Cloudflare"]')].find(img=>img.complete&&img.naturalWidth>0&&Math.round(img.parentElement.getBoundingClientRect().width)===size));
   assert(img.src===expected,"Native image bypassed verified cache");assert(getComputedStyle(img).padding==="0px","Logo retained internal padding");
   assert(Math.max(img.naturalWidth,img.naturalHeight)<=128,"Cached artwork exceeds 128px");
   assert(Math.abs(img.naturalWidth/img.naturalHeight-1200/628)<0.03,"Cached artwork changed aspect ratio");
   const canvas=document.createElement("canvas");canvas.width=size;canvas.height=size;const ctx=canvas.getContext("2d");
   const scale=(size-2)/img.naturalWidth,height=img.naturalHeight*scale;ctx.drawImage(img,1,(size-height)/2,size-2,height);
   const pixels=ctx.getImageData(0,0,size,size).data;let orange=0;for(let i=0;i<pixels.length;i+=4){if(pixels[i]>180&&pixels[i+1]>60&&pixels[i+1]<200&&pixels[i+2]<110&&pixels[i+3]>100)orange++;}
   assert(orange>4,"Cloudflare cloud is not visibly represented at native logo size: "+orange);return {size,orange,width:img.naturalWidth,height:img.naturalHeight,cachedBytes:validateAsset(img.src)};
 }
 try {
  if(sessionStorage.getItem("branding-reload")){
   const jobs=(await invoke("list_jobs",{filter:{}})).items;const saved=jobs.find(job=>job.external_id==="42");
   assert(saved.notes==="Protected branding note"&&saved.favorited&&saved.status==="applied","Refresh lost protected state");
   assert(saved.company_logo.cached_asset_reference===reference&&saved.company_logo.diagnostic.includes("last verified"),"Offline restart lost verified artwork or diagnostics");
   const button=await wait(()=>[...document.querySelectorAll("button")].find(button=>/^DISCOVERED/.test(button.textContent.trim())));button.click();const pixels=await visibleLogo(24);
   await invoke("delete_source",{id:sessionStorage.getItem("branding-source")});const retained=await invoke("get_job",{id:saved.id});assert(retained.source_id===null&&retained.company_logo.cached_asset_reference===reference,"Source deletion erased saved branding");
   await invoke("plugin:event|emit",{event:"native-smoke-result",payload:{ok:true,steps:["Actual sidecar logo extraction, employer identity and one shared asset","Native 24px list and 32px inspector decoding with visible Cloudflare pixels","Offline refresh/reload, protected state and source deletion"],pixels}});return;
  }
  await invoke("set_scheduler_paused",{paused:true});await invoke("update_setting",{key:"notifications.enabled",value:false});
  const source=await invoke("save_workflow",{id:null,spec:{version:4,metadata:{name:"Engineering jobs",company:"Cloudflare"},extractor:{kind:"jsonld",url:${JSON.stringify(origin+"/careers")}},max_jobs_per_run:2,schedule:{cadence:"manual",timezone:"UTC"}}});
  const run=await invoke("run_source_now",{id:source.id});assert(run.status==="success"&&run.counts.inserted===2,"Native branding ingestion failed: "+JSON.stringify(run));
  const jobs=(await invoke("list_jobs",{filter:{}})).items;reference=jobs[0]?.company_logo?.cached_asset_reference;
  assert(reference?.startsWith("sha256:"),"Verified artwork lacks its content reference");
  expected=await invoke("get_company_logo_asset",{assetReference:reference});validateAsset(expected);
  sessionStorage.setItem("branding-asset",expected);sessionStorage.setItem("branding-reference",reference);
  assert(jobs.every(job=>job.company==="Cloudflare"&&job.company_logo.cached_asset_reference===reference),"Workflow name became employer or assets were not shared");
  const saved=jobs.find(job=>job.external_id==="42");assert(saved.company_logo.evidence.url===${JSON.stringify(origin+"/logo.png")},"Original evidence URL changed");
  const discovered=await wait(()=>[...document.querySelectorAll("button")].find(button=>/^DISCOVERED/.test(button.textContent.trim())));discovered.click();await visibleLogo(24);
  const title=await wait(()=>[...document.querySelectorAll("button")].find(button=>button.textContent.trim()==="Native branding engineer 42"));title.click();await visibleLogo(32);
  await invoke("update_job_notes",{id:saved.id,notes:"Protected branding note"});await invoke("set_job_favorite",{id:saved.id,favorited:true});await invoke("update_job_status",{id:saved.id,status:"applied"});
  await fetch(${JSON.stringify(origin+"/offline")});const refresh=await invoke("run_source_now",{id:source.id});assert(refresh.status==="success_with_warnings"&&refresh.counts.updated===2&&refresh.counts.failed===0,"Optional image failure changed ingestion outcome incorrectly: "+JSON.stringify(refresh));
  sessionStorage.setItem("branding-reload","true");sessionStorage.setItem("branding-source",source.id);location.reload();
 }catch(error){await invoke("plugin:event|emit",{event:"native-smoke-result",payload:{ok:false,error:String(error)}});}
})();`);
try {
 const result=await runNativeDev(directory);
 if(result.code!==0||result.signal)throw new Error(`Native branding failed; artifacts ${directory}\n${result.output}`);
 const completion=JSON.parse(await readFile(join(directory,"result.json"),"utf8"));if(!completion.ok)throw new Error(JSON.stringify(completion));
 if(downloads!==2)throw new Error(`Expected one shared asset download per run and none from WebView/offline cache; got ${downloads}`);
 console.log(`Native branding passed; ${downloads} worker asset attempts, no WebView network dependency. Original ${original.length} bytes; cached ${JSON.stringify(completion.pixels)}. Artifacts: ${directory}`);
} finally{server.closeAllConnections();server.close();}
