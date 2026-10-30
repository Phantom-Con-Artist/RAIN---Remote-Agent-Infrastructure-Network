// Uses the isolated RAIN_HOME=/tmp/rain-private-ui hub on port 17901.
const {chromium}=await import(process.env.RAIN_PLAYWRIGHT_MODULE||'playwright');
import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
const run=promisify(execFile);
const browser=await chromium.launch({...(process.env.RAIN_BROWSER?{executablePath:process.env.RAIN_BROWSER}:{}),headless:true,args:['--no-sandbox']});
const checks=[];
try {
 const context=await browser.newContext();const page=await context.newPage();
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto('http://127.0.0.1:17901/#network/connections');
 await page.getByLabel('Hub address',{exact:true}).waitFor();
 await page.getByRole('heading',{name:'Your server hub',exact:true}).waitFor();
 const input=page.getByLabel('Hub address',{exact:true}),submit=page.getByRole('button',{name:'Connect to hub',exact:true});
 for(const [value,expected] of [['http://outside.example','Remote hubs require HTTPS'],['https://name:password@hub.example','without credentials'],['https://hub.example/path','without credentials'],['https://hub.example?token=secret','without credentials'],['javascript:alert(1)','Use an HTTPS']]){
  await input.fill(value);await submit.click();await page.locator('#hub-address-error').filter({hasText:expected}).waitFor();
  assert.equal(await page.evaluate(()=>localStorage.getItem('rain_host_url')),null);
 }
 checks.push('Unsafe or malformed hub addresses rejected without changing current hub');
 await input.fill('http://127.0.0.1:1');await submit.click();await page.locator('#hub-address-error').filter({hasText:'Cannot reach'}).waitFor();assert.equal(await input.inputValue(),'http://127.0.0.1:1');checks.push('Unreachable hub preserves input and previous connection');
 await input.fill('http://127.0.0.1:17901');
 await page.route('**/health',r=>r.fulfill({status:401,body:'Sign in'}));await submit.click();await page.locator('#hub-address-error').filter({hasText:'sign in'}).waitFor();await page.unroute('**/health');
 await page.route('**/health',async r=>{await new Promise(resolve=>setTimeout(resolve,7000));await r.fulfill({status:200,json:{success:true,data:{server_time:'fixture'}}}).catch(()=>{});});await submit.click();await page.locator('#hub-address-error').filter({hasText:'timed out'}).waitFor({timeout:10000});await page.unroute('**/health');
 checks.push('Sign-in required and slow hub responses show recovery guidance');
 await page.route('**/health',r=>r.fulfill({status:403,json:{success:false}}));await submit.click();await page.locator('#hub-address-error').filter({hasText:'denied access'}).waitFor();await page.unroute('**/health');
 await page.route('**/health',r=>r.fulfill({status:200,contentType:'text/html',body:'<h1>Sign in</h1>'}));await submit.click();await page.locator('#hub-address-error').filter({hasText:'not a ready RAIN'}).waitFor();await page.unroute('**/health');checks.push('Denied access and non-RAIN server responses explained');
 for(const width of [1440,768,390,320]){
  await page.setViewportSize({width,height:900});await page.screenshot({path:`/tmp/rain-connections-${width}.png`,fullPage:true});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`overflow at ${width}`);
 }
 checks.push('Connections layout fits 1440, 768, 390 and 320 px');
 await page.evaluate(()=>document.documentElement.setAttribute('data-theme','dark'));await page.waitForTimeout(250);await page.screenshot({path:'/tmp/rain-connections-dark.png',fullPage:true});await page.evaluate(()=>document.documentElement.setAttribute('data-theme','light'));
 await input.fill('http://127.0.0.1:17901');await input.focus();await page.keyboard.press('Tab');assert.equal(await submit.evaluate(el=>el===document.activeElement),true);checks.push('Keyboard form order and dark theme reviewed');

 await page.route('**/api/v1/conversations',r=>r.abort('connectionfailed'));
 await page.locator('.hub-connection-notice').waitFor({timeout:12000});
 await page.unroute('**/api/v1/conversations');await page.getByRole('button',{name:'Retry connection',exact:true}).click();await page.locator('.hub-connection-notice').waitFor({state:'detached'});checks.push('Offline banner appears and recovers after retry');
 await page.evaluate(()=>{localStorage.setItem('rain_auth_host','http://127.0.0.1:17901');localStorage.setItem('rain_auth_token','dummy-secret')});await page.reload();
 await page.getByLabel('Hub address',{exact:true}).fill('http://localhost:17901');await page.getByRole('button',{name:'Connect to hub',exact:true}).click();await page.waitForEvent('load');await page.getByLabel('Hub address',{exact:true}).waitFor();assert.equal(await page.evaluate(()=>localStorage.getItem('rain_host_url')),'http://localhost:17901');assert.equal(await page.evaluate(()=>localStorage.getItem('rain_auth_token')),'');checks.push('Switching hubs clears prior credentials and reloads state');
 await page.evaluate(()=>localStorage.setItem('rain_host_url','javascript:bad'));await page.reload();await page.getByLabel('Hub address',{exact:true}).waitFor();assert.equal(await page.getByLabel('Hub address',{exact:true}).inputValue(),'http://127.0.0.1:17901');checks.push('Corrupt saved address recovers to current origin');
 await run('bin/rain-server',['stop'],{env:{...process.env,RAIN_HOME:'/tmp/rain-private-ui'}});
 try {
  await page.locator('.hub-connection-notice').waitFor({timeout:12000});
  await page.screenshot({path:'/tmp/rain-hub-offline-mobile.png',fullPage:true});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'offline layout overflow');
  const banner=await page.locator('.hub-connection-notice').boundingBox();const main=await page.locator('#main-content').boundingBox();assert.ok(main.y>=banner.y+banner.height,'offline banner must sit above workspace');
 } finally {await run('bin/rain-server',['start','--port','17901'],{env:{...process.env,RAIN_HOME:'/tmp/rain-private-ui'}});}
 await page.getByRole('button',{name:'Retry connection',exact:true}).click();await page.locator('.hub-connection-notice').waitFor({state:'detached'});
 checks.push('Real hub shutdown shows offline state; restart and retry reconnect without client becoming a server');
 assert.deepEqual(errors,[]);console.log(JSON.stringify({checks,pageErrors:errors},null,2));
} finally {await browser.close()}
