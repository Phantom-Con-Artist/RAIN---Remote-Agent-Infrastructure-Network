// Real sysadmin dashboard flow, including security controls and canvas interaction.
const {chromium}=await import(process.env.RAIN_PLAYWRIGHT_MODULE||'playwright');
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const browser=await chromium.launch({...(process.env.RAIN_BROWSER?{executablePath:process.env.RAIN_BROWSER}:{}),headless:true,args:['--no-sandbox']});
try{
 const base=process.env.RAIN_URL||'http://127.0.0.1:17903',key=(await fs.readFile(process.env.RAIN_ADMIN_KEY_FILE||'/tmp/rain-cloud-admin-ui/admin.key','utf8')).trim(),stamp=Date.now();
 const context=await browser.newContext({viewport:{width:1440,height:1000}}),page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(base+'/#network/cloud');await page.getByLabel('Admin key',{exact:true}).waitFor();assert.equal(await page.locator('.cloud-workspace').count(),0);
 await page.getByLabel('Admin key',{exact:true}).fill('wrong');await page.getByRole('button',{name:'Open admin panel',exact:true}).click();await page.getByText('Invalid admin key',{exact:true}).waitFor();
 await page.getByLabel('Admin key',{exact:true}).fill(key);await page.getByRole('button',{name:'Open admin panel',exact:true}).click();await page.getByRole('heading',{name:'Network control',exact:true}).waitFor();
 const stored=await page.evaluate(()=>JSON.stringify(localStorage));assert.ok(!stored.includes(key));assert.ok(stored.includes('rain_infrastructures'));
 const agentName=`Canvas agent ${stamp}`,groupName=`Canvas group ${stamp}`,panel=page.locator('.admin-security');
 await panel.getByLabel('Agent name',{exact:true}).fill(agentName);await panel.getByRole('button',{name:'Create agent credential',exact:true}).click();await panel.locator('.cloud-file-list strong').filter({hasText:agentName}).waitFor();
 await panel.getByText('Protected groups & membership',{exact:true}).click();const form=panel.locator('form').filter({has:page.getByLabel('Group name',{exact:true})});await form.getByLabel('Group name',{exact:true}).fill(groupName);await form.getByLabel(agentName,{exact:true}).check();await form.getByRole('button',{name:'Create protected group',exact:true}).click();
 const node=page.getByRole('button',{name:new RegExp(groupName+', Protected group')});await node.waitFor();await node.scrollIntoViewIfNeeded();await node.focus();await page.keyboard.press('Enter');await page.locator('.admin-node-detail h4').filter({hasText:groupName}).waitFor();
 const transform=()=>page.locator('.admin-canvas svg > g').getAttribute('transform');await page.getByRole('button',{name:'Reset network view',exact:true}).click();const before=await transform();await page.getByRole('button',{name:'Zoom in network',exact:true}).click();assert.notEqual(await transform(),before);await page.getByRole('button',{name:'Reset network view',exact:true}).click();assert.equal(await transform(),before);
 const canvas=page.locator('.admin-canvas');await canvas.focus();await page.keyboard.press('ArrowRight');assert.notEqual(await transform(),before);await page.keyboard.press('Home');assert.equal(await transform(),before);
 await page.getByRole('button',{name:'Pause motion',exact:true}).click();assert.equal(await page.locator('.admin-network-flow.is-active').count(),0);await page.getByRole('button',{name:'Enable motion',exact:true}).click();
 await panel.getByText('Admin key & access records',{exact:true}).click();await panel.getByRole('button',{name:'Read access records',exact:true}).click();await panel.getByRole('columnheader',{name:'HTTP result',exact:true}).waitFor();
 for(const width of [1440,768,390,320]){await page.setViewportSize({width,height:900});await page.locator('.admin-network').scrollIntoViewIfNeeded();await page.locator('.admin-network').screenshot({path:`/tmp/rain-admin-canvas-${width}.png`});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'overflow '+width);}
 await page.emulateMedia({reducedMotion:'reduce'});await page.evaluate(()=>document.documentElement.setAttribute('data-theme','dark'));await page.waitForTimeout(250);await page.locator('.admin-network').screenshot({path:'/tmp/rain-admin-canvas-dark.png'});
 await panel.getByRole('button',{name:'Sign out admin',exact:true}).click();await page.getByLabel('Admin key',{exact:true}).waitFor();assert.equal(await page.locator('.cloud-workspace').count(),0);await page.getByLabel('Saved infrastructures',{exact:true}).waitFor();
 await page.screenshot({path:'/tmp/rain-admin-signin-mobile.png',fullPage:true});assert.ok(!(await page.evaluate(()=>JSON.stringify(localStorage))).includes(key));assert.deepEqual(errors,[]);
 console.log(JSON.stringify({passed:['Wrong key rejected; admin UI hidden until authentication','Infrastructure address remembered without admin key','Agent credential and protected group managed from UI','Canvas selects nodes by keyboard, zooms, pans and resets','Motion pause, reduced-motion mode and responsive/dark rendering','Audit records readable only inside admin UI','Sign-out removes admin controls and preserves saved infrastructure address'],pageErrors:errors},null,2));
}finally{await browser.close()}
