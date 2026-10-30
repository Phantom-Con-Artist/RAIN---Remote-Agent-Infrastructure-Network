// Run against the disposable review hub and its explicitly started local worker.
const {chromium}=await import(process.env.RAIN_PLAYWRIGHT_MODULE||'playwright');
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const browser=await chromium.launch({...(process.env.RAIN_BROWSER?{executablePath:process.env.RAIN_BROWSER}:{}),headless:true,args:['--no-sandbox']});
const checks=[],base=process.env.RAIN_URL||'http://127.0.0.1:17901',stamp=Date.now();
try {
 const page=await browser.newPage({viewport:{width:1440,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(base+'/#network/cloud');await page.getByLabel('Admin key',{exact:true}).waitFor();if(await page.getByLabel('Admin key',{exact:true}).count()){await page.getByLabel('Admin key',{exact:true}).fill(await fs.readFile(process.env.RAIN_ADMIN_KEY_FILE||'/tmp/rain-private-ui/admin.key','utf8'));await page.getByRole('button',{name:'AUTHENTICATE_ADMIN',exact:true}).click();}await page.getByRole('heading',{name:'Your personal cloud',exact:true}).waitFor();
 await page.getByRole('cell',{name:'This hub',exact:true}).waitFor();
 const workerRow=page.getByRole('row').filter({has:page.getByRole('cell',{name:'This hub',exact:true})});
 if(await workerRow.getByRole('button',{name:'RESUME_SHARING',exact:true}).count())await workerRow.getByRole('button',{name:'RESUME_SHARING',exact:true}).click();
 await page.getByLabel('Upload shared file').setInputFiles({name:`phone-note-${stamp}.txt`,mimeType:'text/plain',buffer:Buffer.from('Phone shared file fixture')});
 await page.locator('.cloud-file-list strong').filter({hasText:`phone-note-${stamp}.txt`}).waitFor();
 checks.push('Upload from client adds a real file to the durable shared library');
 const form=page.locator('.cloud-task-form'),title=`Browser task ${stamp}`;
 await form.getByLabel('Task name', {exact:true}).fill(title);
 await form.getByLabel('Command arguments').fill(JSON.stringify(['python3','-c',"import pathlib,os;source=next(pathlib.Path(os.environ['RAIN_INPUT_DIR']).iterdir());pathlib.Path(os.environ['RAIN_OUTPUT_DIR'],'browser-result.txt').write_text(source.read_text()+' processed');print('Browser worker completed')"]));
 await form.getByLabel('Input file (optional)',{exact:true}).selectOption({label:`phone-note-${stamp}.txt`});
 await form.getByRole('button',{name:'DISPATCH_TASK',exact:true}).click();
 const result=page.locator('.cloud-task-list article').filter({has:page.getByRole('heading',{name:title,exact:true})});
 await result.getByText('completed',{exact:true}).waitFor({timeout:30000});await result.getByText('Browser worker completed',{exact:true}).waitFor();
 const downloadEvent=page.waitForEvent('download');await result.getByRole('button',{name:'browser-result.txt',exact:true}).click();const download=await downloadEvent;
 assert.equal(await fs.readFile(await download.path(),'utf8'),'Phone shared file fixture processed');checks.push('Client submits work with input, worker executes, result appears, verified output downloads');
 await form.getByLabel('Task name',{exact:true}).fill(`Browse fixture ${stamp}`);await form.getByLabel('Action',{exact:true}).selectOption('list_files');await form.getByRole('button',{name:'DISPATCH_TASK',exact:true}).click();
 const listing=page.locator('.cloud-task-list article').filter({has:page.getByRole('heading',{name:`Browse fixture ${stamp}`,exact:true})});await listing.locator('.cloud-directory').filter({hasText:'browser-note.txt'}).waitFor({timeout:30000});checks.push('Remote folder task renders the real worker shared-folder contents');
 await workerRow.getByRole('button',{name:'PAUSE_SHARING',exact:true}).click();await workerRow.getByText('Paused',{exact:true}).waitFor();
 await form.getByLabel('Task name',{exact:true}).fill(`Queued while paused ${stamp}`);await form.getByRole('button',{name:'DISPATCH_TASK',exact:true}).click();
 const queued=page.locator('.cloud-task-list article').filter({has:page.getByRole('heading',{name:`Queued while paused ${stamp}`,exact:true})});await queued.getByText('queued · waiting for an eligible worker',{exact:true}).waitFor();await queued.getByRole('button',{name:'CANCEL_TASK',exact:true}).click();await queued.getByText('cancelled',{exact:true}).waitFor();await workerRow.getByRole('button',{name:'RESUME_SHARING',exact:true}).click();checks.push('UI pauses worker resource sharing, queues work and cancels it');
 await page.getByLabel('Worker name',{exact:true}).fill(`Invited computer ${stamp}`);await page.getByRole('button',{name:'ISSUE_JOIN_TOKEN',exact:true}).click();await page.locator('.cloud-invitation code').first().waitFor();assert.equal((await page.locator('.cloud-invitation .connection-address code').innerText()).length,64);await page.getByRole('button',{name:'HIDE_JOIN_TOKEN',exact:true}).click();checks.push('Owner creates and hides a one-use worker invitation');
 await form.getByLabel('Task name',{exact:true}).fill('Draft survives a refresh');await page.getByRole('button',{name:'Refresh personal cloud',exact:true}).click();await page.waitForTimeout(600);assert.equal(await form.getByLabel('Task name',{exact:true}).inputValue(),'Draft survives a refresh');
 for(const width of [1440,768,390,320]){await page.setViewportSize({width,height:900});await page.screenshot({path:`/tmp/rain-cloud-${width}.png`,fullPage:true});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`Page overflow at ${width}`)}
 await page.evaluate(()=>document.documentElement.setAttribute('data-theme','dark'));await page.waitForTimeout(250);await page.screenshot({path:'/tmp/rain-cloud-dark.png',fullPage:true});checks.push('Draft preserved during refresh; mobile widths and dark mode rendered');
 assert.deepEqual(errors,[]);console.log(JSON.stringify({checks,pageErrors:errors},null,2));
} finally {await browser.close()}
