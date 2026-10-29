import React, {useEffect, useRef, useState} from 'react';
import {Terminal} from '@xterm/xterm';
import {FitAddon} from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import {apiRequest} from '../api';

interface Output {sequence:number;chunks:string[];reset:boolean;interactive:boolean;status:string}

export default function TerminalSurface({id,onClose}: {id:string;onClose?:()=>void}) {
 const container=useRef<HTMLDivElement>(null);
 const terminal=useRef<Terminal|null>(null);
 const sendRef=useRef<(data:string)=>void>(()=>{});
 const [error,setError]=useState('');
 const [draft,setDraft]=useState('');
 const [running,setRunning]=useState(false);
 const interactive=useRef(true);
 useEffect(()=>{
  if(!container.current)return;
  let disposed=false, sequence=0, timer:ReturnType<typeof setTimeout>, status='running';
  let sendQueue=Promise.resolve();
  const theme=()=>{const style=getComputedStyle(document.documentElement);return {background:style.getPropertyValue('--bg-canvas').trim(),foreground:style.getPropertyValue('--text-primary').trim(),cursor:style.getPropertyValue('--text-primary').trim(),selectionBackground:style.getPropertyValue('--bg-active').trim(),blue:'#888888',brightBlue:'#bbbbbb',cyan:'#999999',brightCyan:'#cccccc'}};
  const term=new Terminal({cursorBlink:false,convertEol:false,fontSize:13,fontFamily:'ui-monospace, monospace',scrollback:5000,theme:theme(),allowProposedApi:false,screenReaderMode:true});
  const fit=new FitAddon();term.loadAddon(fit);term.open(container.current);terminal.current=term;
  // Serialize keystrokes so concurrent HTTP requests cannot reorder input.
  const send=(input:string)=>{sendQueue=sendQueue.then(async()=>{if(disposed)return;await apiRequest(`/api/v1/agents/${id}/input`,{method:'POST',body:JSON.stringify({input})});if(!disposed)setError('')}).catch(e=>{if(!disposed)setError(e.message)});};
  sendRef.current=send;
  const data=term.onData(send);
  const resized=term.onResize(size=>{if(status==='running')apiRequest(`/api/v1/agents/${id}/resize`,{method:'POST',body:JSON.stringify(size)}).catch(()=>{});});
  const observer=new ResizeObserver(()=>{if(!disposed&&container.current?.clientWidth)fit.fit()});observer.observe(container.current);
  const colors=new MutationObserver(()=>{term.options.theme=theme()});colors.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});
  async function poll(){
   try{const r=await apiRequest<Output>(`/api/v1/agents/${id}/output?after=${sequence}`);if(disposed)return;status=r.data.status;interactive.current=r.data.interactive;setRunning(status==='running');if(r.data.reset)term.reset();for(const chunk of r.data.chunks)term.write(chunk);sequence=r.data.sequence;term.options.disableStdin=status!=='running';}
   catch(e:any){if(!disposed)setError(e.message)}
   finally{if(!disposed)timer=setTimeout(poll,200)}
  }
  fit.fit();poll();
  return()=>{disposed=true;clearTimeout(timer);data.dispose();resized.dispose();observer.disconnect();colors.disconnect();term.dispose();terminal.current=null;};
 },[id]);
 return <section className="terminal-session" aria-label="Interactive terminal">
  <div className="terminal-toolbar"><strong>Interactive terminal</strong><span>{running?'Native CLI · keyboard and paste supported':'Session stopped · saved output'}</span>{onClose&&<button className="icon-button" onClick={onClose}>Close terminal</button>}</div>
  <div ref={container} className="terminal-canvas"/>
  <form className="terminal-prompt" onSubmit={e=>{e.preventDefault();sendRef.current(draft+(interactive.current?'\r':'\n'));setDraft('');terminal.current?.focus()}}><label className="sr-only" htmlFor={`terminal-prompt-${id}`}>Send a prompt to the CLI</label><input id={`terminal-prompt-${id}`} className="chat-input-field" disabled={!running} value={draft} onChange={e=>setDraft(e.target.value)} placeholder="Send a prompt or /command…"/><button className="icon-button" type="submit" disabled={!running}>Send</button></form>
  <div className="terminal-keybar">{[['Interrupt','\x03'],['Escape','\x1b'],['Tab','\t'],['Up','\x1b[A'],['Down','\x1b[B'],['Enter','\r']].map(([label,key])=><button className="icon-button" key={label} disabled={!running} onClick={()=>{sendRef.current(key);terminal.current?.focus()}}>{label}</button>)}</div>
  {error&&<p role="alert">{error}</p>}
 </section>
}
