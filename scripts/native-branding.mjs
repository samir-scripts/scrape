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
  if(request.url==="/employer-logo.png"){downloads++;response.statusCode=online?200:404;response.end(online?original:"Authored unavailable image");return;}
  if(request.url==="/robots.txt"){response.end("User-agent: *\nAllow: /");return;}
  if(request.url==="/careers"){
    response.end(`<script type="application/ld+json">${JSON.stringify([42,43].map(id=>({"@type":"JobPosting",identifier:String(id),title:`Native branding engineer ${id}`,url:`${origin}/jobs/${id}`,hiringOrganization:{name:"Native Fixture Co"}})))}</script>`);
    return;
  }
  if(request.url?.startsWith("/jobs/")){
    response.end('<img alt="Native Fixture Co Logo" src="/employer-logo.png">');
    return;
  }
  response.statusCode=404;response.end("Not found");
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
   const img=await wait(()=>[...document.querySelectorAll('img[alt="Native Fixture Co"]')].find(img=>img.complete&&img.naturalWidth>0&&Math.round(img.parentElement.getBoundingClientRect().width)===size));
   assert(img.src===expected,"Native image bypassed verified cache");assert(getComputedStyle(img).padding==="0px","Logo retained internal padding");
   assert(Math.max(img.naturalWidth,img.naturalHeight)<=128,"Cached artwork exceeds 128px");
   assert(Math.abs(img.naturalWidth/img.naturalHeight-1200/628)<0.03,"Cached artwork changed aspect ratio");
   const canvas=document.createElement("canvas");canvas.width=size;canvas.height=size;const ctx=canvas.getContext("2d");
   const scale=(size-2)/img.naturalWidth,height=img.naturalHeight*scale;ctx.drawImage(img,1,(size-height)/2,size-2,height);
   const pixels=ctx.getImageData(0,0,size,size).data;let orange=0;for(let i=0;i<pixels.length;i+=4){if(pixels[i]>180&&pixels[i+1]>60&&pixels[i+1]<200&&pixels[i+2]<110&&pixels[i+3]>100)orange++;}
   assert(orange>4,"Employer artwork is not visibly represented at native logo size: "+orange);return {size,orange,width:img.naturalWidth,height:img.naturalHeight,cachedBytes:validateAsset(img.src)};
 }
 try {
  if(sessionStorage.getItem("branding-reload")){
   const jobs=(await invoke("list_jobs",{filter:{}})).items;const saved=jobs.find(job=>job.external_id==="42");
   assert(saved.company==="Native Fixture Co"&&saved.source_id===sessionStorage.getItem("branding-source"),"Logo relationship lost source or employer identity");
   assert(saved.notes==="Protected branding note"&&saved.favorited&&saved.status==="applied","Refresh lost protected state");
   assert(saved.company_logo.cached_asset_reference===reference&&saved.company_logo.diagnostic.includes("last verified"),"Offline restart lost verified artwork or diagnostics");
   const button=await wait(()=>[...document.querySelectorAll("button")].find(button=>/^DISCOVERED/.test(button.textContent.trim())));button.click();const pixels=await visibleLogo(24);
   await invoke("delete_source",{id:sessionStorage.getItem("branding-source")});const retained=await invoke("get_job",{id:saved.id});assert(retained.source_id===null&&retained.company_logo.cached_asset_reference===reference,"Source deletion erased saved branding");
   await invoke("plugin:event|emit",{event:"native-smoke-result",payload:{ok:true,steps:["Visible persisted job automatically resolved employer-labelled artwork through native IPC","Exact source/employer relationship and verified asset persisted across reopen","Offline refresh, protected state and source deletion retained cached artwork"],pixels}});return;
  }
  await invoke("set_scheduler_paused",{paused:true});await invoke("update_setting",{key:"notifications.enabled",value:false});
  const source=await invoke("save_workflow",{id:null,spec:{version:4,metadata:{name:"Fictional employer jobs",company:"Native Fixture Co"},extractor:{kind:"jsonld",url:${JSON.stringify(origin+"/careers")}},max_jobs_per_run:2,schedule:{cadence:"manual",timezone:"UTC"}}});
  const run=await invoke("run_source_now",{id:source.id});assert(run.status==="success"&&run.counts.inserted===2,"Native branding ingestion failed: "+JSON.stringify(run));
  let jobs=(await invoke("list_jobs",{filter:{}})).items;const saved=jobs.find(job=>job.external_id==="42");
  assert(saved?.company==="Native Fixture Co"&&!saved.company_logo?.cached_asset_reference,"Fixture must enter the database without logo artwork");
  const discovered=await wait(()=>[...document.querySelectorAll("button")].find(button=>/^DISCOVERED/.test(button.textContent.trim())));discovered.click();
  assert(!document.body.innerText.includes("Fetch logo"),"Manual Fetch logo action remains visible");
  let persisted;for(let count=0;count<100;count++){persisted=await invoke("get_job",{id:saved.id});if(persisted.company_logo?.status==="verified"&&persisted.company_logo.cached_asset_reference)break;await pause(50);}
  reference=persisted?.company_logo?.cached_asset_reference;
  assert(reference?.startsWith("sha256:"),"Visible job lookup did not persist verified artwork");
  assert(persisted.source_id===source.id&&persisted.company==="Native Fixture Co","Logo was persisted to a different source/employer relationship");
  expected=await invoke("get_company_logo_asset",{assetReference:reference});validateAsset(expected);
  sessionStorage.setItem("branding-asset",expected);sessionStorage.setItem("branding-reference",reference);
  await visibleLogo(24);
  jobs=(await invoke("list_jobs",{filter:{}})).items;assert(jobs.every(job=>job.company==="Native Fixture Co"&&job.source_id===source.id),"Job identity changed during branding lookup");
  const title=await wait(()=>[...document.querySelectorAll("button")].find(button=>button.textContent.trim()==="Native branding engineer 42"));title.click();await visibleLogo(32);
  await invoke("update_job_notes",{id:saved.id,notes:"Protected branding note"});await invoke("set_job_favorite",{id:saved.id,favorited:true});await invoke("update_job_status",{id:saved.id,status:"applied"});
  await fetch(${JSON.stringify(origin+"/offline")});const refresh=await invoke("run_source_now",{id:source.id});assert(["success","success_with_warnings"].includes(refresh.status)&&refresh.counts.updated===2&&refresh.counts.failed===0,"Offline refresh changed ingestion outcome incorrectly: "+JSON.stringify(refresh));
  sessionStorage.setItem("branding-reload","true");sessionStorage.setItem("branding-source",source.id);location.reload();
 }catch(error){await invoke("plugin:event|emit",{event:"native-smoke-result",payload:{ok:false,error:String(error)}});}
})();`);
try {
 const result=await runNativeDev(directory);
 if(result.code!==0||result.signal)throw new Error(`Native branding failed; artifacts ${directory}\n${result.output}`);
 const completion=JSON.parse(await readFile(join(directory,"result.json"),"utf8"));if(!completion.ok)throw new Error(JSON.stringify(completion));
 if(downloads!==1)throw new Error(`Expected one automatic employer asset download and none from WebView/reload; got ${downloads}`);
 console.log(`Native automatic branding passed; ${downloads} worker employer asset downloads. Original ${original.length} bytes; cached ${JSON.stringify(completion.pixels)}. Artifacts: ${directory}`);
} finally{server.closeAllConnections();server.close();}
