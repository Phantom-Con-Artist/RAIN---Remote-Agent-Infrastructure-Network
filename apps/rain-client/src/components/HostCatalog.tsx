import React,{useEffect,useState} from 'react';
import {apiRequest} from '../api';
interface Executable {name:string;command:string;chat_provider?:string}
interface Catalog {agents:Executable[];executables:Executable[];shell:string|null;home:string}
export function HostCatalog({machine,cwd,onLaunch}: {machine:string;cwd:string;onLaunch:(command:string,name:string,args:string[],provider?:string)=>Promise<void>}) {
 const [catalog,setCatalog]=useState<Catalog|null>(null),[error,setError]=useState(''),[loading,setLoading]=useState(false),[busy,setBusy]=useState(''),[search,setSearch]=useState(''),[revision,setRevision]=useState(0);
 useEffect(()=>{let active=true;setCatalog(null);setError('');setLoading(true);apiRequest<Catalog>(`/api/v1/agents/catalog?machine=${encodeURIComponent(machine)}`).then(r=>{if(active)setCatalog(r.data)}).catch(e=>{if(active)setError(e.message)}).finally(()=>{if(active)setLoading(false)});return()=>{active=false};},[machine,revision]);
 async function launch(item:Executable,args:string[]=[]){setBusy(item.command);setError('');try{await onLaunch(item.command,item.name,args,item.chat_provider)}catch(e:any){setError(e.message)}finally{setBusy('')}}
 const matches=search.trim()?catalog?.executables.filter(e=>e.name.toLowerCase().includes(search.toLowerCase())).slice(0,40):[];
 return <section className="host-catalog" aria-label="Installed CLI tools">
  <div className="section-header-row"><h3>Installed agent CLIs</h3><button className="icon-button" disabled={loading} onClick={()=>setRevision(v=>v+1)}>{loading?'Scanning…':'Scan again'}</button></div>
  <p>Launch on {machine||'this host'}{cwd?` in ${cwd}`:''}. Each click creates an independent session.</p>
  {error&&<p role="alert">{error}</p>}
  <div className="catalog-actions">{catalog?.agents.map(item=><button className="icon-button" disabled={!!busy} key={item.command} title={item.command} onClick={()=>launch(item)}>{item.chat_provider?'Chat with':'Launch'} {item.name}</button>)}
  {catalog?.shell&&<button className="icon-button primary" disabled={!!busy} onClick={()=>launch({name:'Terminal',command:catalog.shell!},['-l'])}>Launch terminal</button>}</div>
  {catalog&&!catalog.agents.length&&<p>No recognized agent CLIs found. Search installed executables below, or use the custom launcher.</p>}
  <label>Find another installed CLI<input className="chat-input-field" value={search} onChange={e=>setSearch(e.target.value)} placeholder="Search executable names" /></label>
  {!!matches?.length && <div className="catalog-actions">{matches.map(item=><button className="icon-button" disabled={!!busy} key={item.command} title={item.command} onClick={()=>launch(item)}>{item.chat_provider?'Chat with':'Launch'} {item.name}</button>)}</div>}
  {busy&&<p role="status">Launching session…</p>}
 </section>
}
