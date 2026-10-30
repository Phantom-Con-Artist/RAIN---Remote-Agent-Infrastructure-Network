const {chromium}=await import(process.env.RAIN_PLAYWRIGHT_MODULE||'playwright');
import assert from 'node:assert/strict';
const b=await chromium.launch({...(process.env.RAIN_BROWSER?{executablePath:process.env.RAIN_BROWSER}:{}),headless:true,args:['--no-sandbox']});
try{
 const origin=process.env.RAIN_HTTPS_URL;
 const context=await b.newContext({ignoreHTTPSErrors:true,httpCredentials:{username:'owner',password:process.env.RAIN_TEST_PASSWORD},viewport:{width:390,height:844}});
 await context.addInitScript(({origin})=>{localStorage.setItem('rain_auth_host',origin);localStorage.setItem('rain_auth_token','dummy-device-token')},{origin});
 const p=await context.newPage();const errors=[];p.on('pageerror',e=>errors.push(e.message));
 await p.goto(origin+'/#network/connections');
 await p.getByLabel('Hub address',{exact:true}).waitFor();
 await p.waitForFunction(()=>document.querySelector('.network-page-heading p')?.textContent!=='Connecting to this hub');
 assert.equal(await p.locator('.hub-connection-notice').count(),0);
 const request=await p.evaluate(async()=>{const response=await fetch('/api/v1/auth/session',{headers:{'X-Rain-Device-Token':'dummy-device-token'}});return {status:response.status,body:await response.json()}});
 assert.equal(request.status,200);assert.equal(request.body.data.authenticated,false);
 await p.getByRole('button',{name:'Connect to hub',exact:true}).click();await p.waitForEvent('load');await p.getByLabel('Hub address',{exact:true}).waitFor();
 await p.screenshot({path:'/tmp/rain-private-https-mobile.png',fullPage:true});assert.deepEqual(errors,[]);
 console.log('PASS: live HTTPS browser login, UI assets, API polling, independent device-token header and reconnect');
}finally{await b.close()}
