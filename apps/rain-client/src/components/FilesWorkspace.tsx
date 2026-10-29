import React,{useEffect,useRef,useState} from 'react';
import {apiRequest} from '../api';
import {FileEntry} from '../types';
import {FilesView} from './FilesView';
import {MachinePicker} from './MachinePicker';
import {TerminalPanel} from './TerminalPanel';
export function FilesWorkspace({machine,onMachineChange,onDirtyChange}:{machine:string;onMachineChange:(machine:string)=>void;onDirtyChange?:(dirty:boolean)=>void}) {
 const [path,setPath]=useState('~'),[files,setFiles]=useState<FileEntry[]>([]),[error,setError]=useState(''),[loading,setLoading]=useState(false),[terminal,setTerminal]=useState<string|null>(null);
 const [launching,setLaunching]=useState(false);
 const generation=useRef(0);
 const suffix=`machine=${encodeURIComponent(machine)}`;
 async function load(next:string){const ticket=++generation.current;setLoading(true);setError('');try{const r=await apiRequest<FileEntry[]>(`/api/v1/fs/list?path=${encodeURIComponent(next)}&${suffix}`);if(ticket===generation.current){setPath(next);setFiles(r.data)}}catch(e:any){if(ticket===generation.current)setError(e.message)}finally{if(ticket===generation.current)setLoading(false)}}
 useEffect(()=>{setFiles([]);setPath('~');setTerminal(null);load('~');return()=>{generation.current++}},[machine]);
 async function launchTerminal(){if(launching)return;setLaunching(true);setError('');try{const catalog=await apiRequest<{shell:string|null}>('/api/v1/agents/catalog?'+suffix);if(!catalog.data.shell)throw new Error('No shell found on this host');const r=await apiRequest<{id:string}>('/api/v1/agents/start',{method:'POST',body:JSON.stringify({name:'Terminal',provider:'custom',command:catalog.data.shell,args:['-l'],interactive:true,machine:machine||null,cwd:path==='~'?null:path,env_vars:{}})});setTerminal(r.data.id)}catch(e:any){setError(e.message)}finally{setLaunching(false)}}
 return <div className="files-workspace">
  <div className="workspace-toolbar"><MachinePicker machine={machine} onChange={onMachineChange}/><button className="icon-button primary" disabled={launching} onClick={launchTerminal}>{launching?'Launching…':'Launch terminal here'}</button></div>
  {loading&&<p role="status">Loading directory…</p>}{error&&<p role="alert">{error}</p>}
  {terminal&&<TerminalPanel id={terminal} onClose={()=>setTerminal(null)}/>}
  <FilesView onDirtyChange={onDirtyChange} key={machine} currentPath={path} files={files} onOpenDirectory={load} onReadFile={async p=>{const r=await apiRequest<{content:string;is_binary:boolean}>(`/api/v1/fs/read?path=${encodeURIComponent(p)}&${suffix}`);return r.data}} onWriteFile={async(p,content)=>{await apiRequest('/api/v1/fs/write?'+suffix,{method:'POST',body:JSON.stringify({path:p,content})})}}/>
 </div>
}
