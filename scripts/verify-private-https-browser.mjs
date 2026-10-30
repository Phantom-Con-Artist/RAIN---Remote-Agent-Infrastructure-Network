const {chromium}=await import(process.env.RAIN_PLAYWRIGHT_MODULE||'playwright');
import assert from 'node:assert/strict';
const browser=await chromium.launch({...(process.env.RAIN_BROWSER?{executablePath:process.env.RAIN_BROWSER}:{}),headless:true,args:['--no-sandbox']});
try{
 const origin=process.env.RAIN_HTTPS_URL,context=await browser.newContext({ignoreHTTPSErrors:true,viewport:{width:390,height:844}}),page=await context.newPage();
 await page.goto(origin+'/#network/cloud');await page.getByLabel('Admin key',{exact:true}).fill(process.env.RAIN_TEST_ADMIN_KEY);await page.getByRole('button',{name:'Open admin panel',exact:true}).click();
 await page.getByRole('heading',{name:'Network control',exact:true}).waitFor();
 assert.equal(await page.locator('#admin-key').count(),0);
 const cookie=(await context.cookies()).find(c=>c.name==='rain_admin');assert.ok(cookie.httpOnly&&cookie.secure&&cookie.sameSite==='Strict');
 assert.equal(await page.evaluate(()=>Object.values(localStorage).includes('rain_admin')),false);
 await page.screenshot({path:'/tmp/rain-private-https-mobile.png',fullPage:true});
 console.log('PASS: HTTPS admin-key sign-in and network dashboard; secure session stays outside JavaScript storage');
}finally{await browser.close()}
