import { readFileSync } from "node:fs";
import { test, expect } from "./fixtures";
import { discoverFixtures } from "./jobFixtures";
import { contrastFailures } from "./contrast";
const artwork="data:image/png;base64,"+readFileSync("crates/scrape-worker/tests/fixtures/cloudflare-logo-cache.png").toString("base64");
for(const theme of ["paper","obsidian"]){
 test(`${theme} employer artwork stays recognizable offline in lists and inspector`,async({page})=>{
  let external=0;await page.route("https://offline.fixture.test/**",route=>{external++;return route.abort();});
  await page.route("**/broken-logo.png",route=>route.fulfill({status:404,body:"missing"}));
  await page.setViewportSize({width:1280,height:900});await discoverFixtures(page);
  await page.evaluate(asset=>{
   const sample=JSON.parse(localStorage.getItem("scrape:demo:v1:jobs")!)[0];
   const canvas=document.createElement("canvas");canvas.width=160;canvas.height=20;const ctx=canvas.getContext("2d")!;ctx.fillStyle="#d07020";ctx.fillRect(10,4,140,12);const wide=canvas.toDataURL("image/png");
   const jobs=["Cloudflare","Wide transparent fixture","Broken fixture","Missing fixture"].map((company,i)=>({...sample,id:`branding-${i}`,dedupe_key:`branding:${i}`,title:`Branding fixture ${i}`,company,status:"viewed",company_icon_url:i===2?"/broken-logo.png":"https://offline.fixture.test/logo.png",company_logo:i===2?null:{evidence:i===3?null:{url:"https://offline.fixture.test/logo.png",page_url:"https://offline.fixture.test/careers",method:"employer_label"},checked_at:"2026-10-16T00:00:00Z",status:i===3?"missing_metadata":"verified",asset:i===3?null:i===1?wide:asset,diagnostic:i===0?"Using last verified logo. Latest lookup unavailable.":null}}));
   localStorage.setItem("scrape:demo:v1:jobs",JSON.stringify(jobs));
  },artwork);await page.reload();
  if(theme==="obsidian"){await page.getByRole("button",{name:"SETTINGS",exact:true}).click();await page.getByRole("button",{name:/^OBSIDIAN/}).click();await page.getByRole("button",{name:/^DISCOVERED/}).click();}
  const logo=page.getByRole("img",{name:"Cloudflare"}).first();await expect(logo).toBeVisible();
  await expect.poll(()=>logo.evaluate(img=>(img as HTMLImageElement).naturalWidth)).toBe(1200);
  const result=await logo.evaluate(img=>{
   const image=img as HTMLImageElement,box=image.parentElement!.getBoundingClientRect();const canvas=document.createElement("canvas");canvas.width=24;canvas.height=24;const ctx=canvas.getContext("2d")!;const h=image.naturalHeight*22/image.naturalWidth;ctx.drawImage(image,1,(24-h)/2,22,h);const pixels=ctx.getImageData(0,0,24,24).data;let orange=0;for(let i=0;i<pixels.length;i+=4){if(pixels[i]>180&&pixels[i+1]>60&&pixels[i+1]<200&&pixels[i+2]<110&&pixels[i+3]>100)orange++;}
   return {width:box.width,height:box.height,padding:getComputedStyle(image).padding,fit:getComputedStyle(image).objectFit,backing:getComputedStyle(image.parentElement!).backgroundColor,orange};
  });expect(result).toMatchObject({width:24,height:24,padding:"0px",fit:"contain",backing:"rgb(250, 250, 250)"});expect(result.orange).toBeGreaterThan(4);
  const wide=page.getByRole("img",{name:"Wide transparent fixture"});await expect.poll(()=>wide.evaluate(img=>(img as HTMLImageElement).naturalWidth)).toBe(160);expect(await wide.evaluate(img=>getComputedStyle(img).objectFit)).toBe("contain");
  await expect(page.getByLabel("Broken fixture initials")).toHaveAttribute("data-logo-status","display_failed");await expect(page.getByLabel("Missing fixture initials")).toHaveAttribute("title",/No employer logo metadata/);
  await page.getByRole("button",{name:"View Branding fixture 0 at Cloudflare",exact:true}).click();
  await expect.poll(()=>page.getByRole("img",{name:"Cloudflare"}).evaluateAll(images=>images.map(img=>Math.round(img.parentElement!.getBoundingClientRect().width)))).toContain(32);
  expect(external).toBe(0);expect(await contrastFailures(page)).toEqual([]);
  await page.screenshot({path:`/tmp/scrape-branding-${theme}-1280.png`});
  await page.setViewportSize({width:480,height:900});await expect(page.getByRole("img",{name:"Cloudflare"}).last()).toBeVisible();await page.screenshot({path:`/tmp/scrape-branding-${theme}-480.png`});
  await page.setViewportSize({width:1280,height:900});await page.evaluate(()=>{document.documentElement.style.zoom="2";});
  await expect.poll(()=>page.getByRole("img",{name:"Cloudflare"}).evaluateAll(images=>images.map(img=>Math.round(img.parentElement!.getBoundingClientRect().width)))).toContain(64);
  await page.screenshot({path:`/tmp/scrape-branding-${theme}-200.png`});
 });
}
