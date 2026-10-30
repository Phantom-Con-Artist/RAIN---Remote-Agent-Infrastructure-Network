import React,{useEffect,useRef,useState} from 'react';
import {apiRequest} from '../api';
import {FileEntry,ChatAttachment} from '../types';
import {AttachmentCard} from './AttachmentCard';
import {FilesView} from './FilesView';
import {MachinePicker} from './MachinePicker';
import {TerminalPanel} from './TerminalPanel';
export function FilesWorkspace({machine,onMachineChange,onDirtyChange}:{machine:string;onMachineChange:(machine:string)=>void;onDirtyChange?:(dirty:boolean)=>void}) {
 const [path,setPath]=useState('~'),[files,setFiles]=useState<FileEntry[]>([]),[error,setError]=useState(''),[loading,setLoading]=useState(false),[terminal,setTerminal]=useState<string|null>(null);
 const [inspected,setInspected]=useState<ChatAttachment|null>(null),[inspecting,setInspecting]=useState(false);
 const [launching,setLaunching]=useState(false);
 const generation=useRef(0),inspectGeneration=useRef(0);
 const suffix=`machine=${encodeURIComponent(machine)}`;
 async function load(next:string){const ticket=++generation.current;setLoading(true);setError('');try{const r=await apiRequest<FileEntry[]>(`/api/v1/fs/list?path=${encodeURIComponent(next)}&${suffix}`);if(ticket===generation.current){setPath(next);setFiles(r.data)}}catch(e:any){if(ticket===generation.current)setError(e.message)}finally{if(ticket===generation.current)setLoading(false)}}
 useEffect(()=>{inspectGeneration.current++;setInspecting(false);setFiles([]);setPath('~');setTerminal(null);setInspected(null);load('~');return()=>{generation.current++}},[machine]);
 async function launchTerminal(){if(launching)return;setLaunching(true);setError('');try{const catalog=await apiRequest<{shell:string|null}>('/api/v1/agents/catalog?'+suffix);if(!catalog.data.shell)throw new Error('No shell found on this host');const r=await apiRequest<{id:string}>('/api/v1/agents/start',{method:'POST',body:JSON.stringify({name:'Terminal',provider:'custom',command:catalog.data.shell,args:['-l'],interactive:true,machine:machine||null,cwd:path==='~'?null:path,env_vars:{}})});setTerminal(r.data.id)}catch(e:any){setError(e.message)}finally{setLaunching(false)}}
 return <div className="files-workspace">
  <div className="workspace-toolbar"><h1>Files</h1><MachinePicker machine={machine} onChange={onMachineChange}/><button className="icon-button primary" disabled={launching} onClick={launchTerminal}>{launching?'Launching…':'Launch terminal here'}</button></div>
  {loading&&<p role="status">Loading directory…</p>}{error&&<p role="alert">{error}</p>}
  {terminal&&<TerminalPanel id={terminal} onClose={()=>setTerminal(null)}/>}
  {inspecting&&<p role="status">Preparing file preview…</p>}
  {inspected&&<section className="file-inspector-panel"><button type="button" className="icon-button" onClick={()=>setInspected(null)}>Close preview</button><AttachmentCard key={inspected.id} file={inspected} initiallyOpen/></section>}
  <FilesView onInspectFile={async file=>{if(inspecting)return;const ticket=++inspectGeneration.current;setInspecting(true);setError('');try{const r=await apiRequest<ChatAttachment>('/api/v1/artifacts/from-file',{method:'POST',body:JSON.stringify({path:file.path,machine:machine||null}),signal:AbortSignal.timeout(300000)});if(ticket===inspectGeneration.current)setInspected(r.data)}catch(e){if(ticket===inspectGeneration.current)setError(e instanceof Error?e.message:'Could not inspect file')}finally{if(ticket===inspectGeneration.current)setInspecting(false)}}} onDirtyChange={onDirtyChange} key={machine} currentPath={path} files={files} onOpenDirectory={load} onReadFile={async p=>{const r=await apiRequest<{content:string;is_binary:boolean}>(`/api/v1/fs/read?path=${encodeURIComponent(p)}&${suffix}`);return r.data}} onWriteFile={async(p,content)=>{await apiRequest('/api/v1/fs/write?'+suffix,{method:'POST',body:JSON.stringify({path:p,content})})}}/>
 </div>
}
