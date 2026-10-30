import {useState,useEffect} from 'react';
import {apiRequest} from '../api';
import type {AgentSession} from '../types';
interface Dispatch{id:string;target_id:string;target_name:string;status:string;error?:string;queued_at:string}
interface Trace{id:string;dispatch_id:string;agent_id:string;step:string;timestamp:string}
export function ChatActivity({id,members}:{id:string;members:AgentSession[]}){
 const [data,setData]=useState<{dispatches:Dispatch[];traces:Trace[]}>({dispatches:[],traces:[]}),[error,setError]=useState(''),[busy,setBusy]=useState(''),[terminate,setTerminate]=useState<string|null>(null);
 useEffect(()=>{let live=true;const controller=new AbortController();const load=()=>apiRequest<typeof data>(`/api/v1/conversations/${id}/activity`,{signal:controller.signal}).then(r=>{if(live){setData(r.data);setError('')}}).catch(e=>{if(live)setError(e.message)});load();const timer=setInterval(load,2000);return()=>{live=false;controller.abort();clearInterval(timer)}},[id]);
 async function control(agent:string,action:string){setBusy(agent);setError('');try{await apiRequest(`/api/v1/conversations/${id}/agent-control`,{method:'POST',body:JSON.stringify({agent_id:agent,action})});setTerminate(null)}catch(e){setError(e instanceof Error?e.message:'Could not stop agent')}finally{setBusy('')}}
 const running=data.dispatches.filter(d=>['running','queued'].includes(d.status));
 return <details className="chat-activity"><summary>Activity <span>{running.length?`${running.length} active ${running.length===1?'turn':'turns'}`:'No active turns'}</span></summary>
 <p className="activity-help">Tool events come from the installed CLI. Providers without tool events show turn status. Interrupt stops this chat’s turn; terminate stops the agent across all chats.</p>
 <div className="activity-members">{members.map(m=><div key={m.id}><strong>{m.name}</strong><span>{running.find(d=>d.target_id===m.id)?.status||m.state}</span><button type="button" className="icon-button" disabled={busy===m.id||!running.some(d=>d.target_id===m.id)} onClick={()=>control(m.id,'interrupt')}>Interrupt</button><button type="button" className="icon-button" disabled={busy===m.id||['stopped','completed','disconnected'].includes(m.state)} onClick={()=>setTerminate(m.id)}>Terminate…</button></div>)}</div>
 {terminate&&<div className="activity-confirm" role="alert"><p>Terminate {members.find(m=>m.id===terminate)?.name}? This stops its process and work in every chat. You can launch a new session from Machines.</p><button type="button" className="icon-button" disabled={!!busy} onClick={()=>control(terminate,'terminate')}>Terminate agent</button><button type="button" className="icon-button" onClick={()=>setTerminate(null)}>Keep running</button></div>}
 {error&&<p role="alert">{error}</p>}
 <ol className="activity-timeline">{data.dispatches.slice(0,12).map(d=><li key={d.id}><div><strong>{d.target_name}</strong> · {d.status} <time>{new Date(d.queued_at).toLocaleTimeString()}</time></div>{d.error&&<p>{d.error}</p>}{data.traces.filter(t=>t.dispatch_id===d.id).reverse().map(t=><div className="activity-step" key={t.id}>{t.step}</div>)}</li>)}</ol>
 </details>
}
