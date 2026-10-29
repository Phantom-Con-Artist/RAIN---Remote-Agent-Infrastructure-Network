import React,{useRef,useState} from 'react';
import {FiPaperclip} from 'react-icons/fi';
import {apiRequest} from '../api';
import {ChatAttachment} from '../types';
import {MachinePicker} from './MachinePicker';
export function AttachmentPicker({disabled,onAttach}:{disabled:boolean;onAttach:(attachment:ChatAttachment)=>void}){
 const fileInput=useRef<HTMLInputElement>(null);
 const [hostPath,setHostPath]=useState(''),[machine,setMachine]=useState(''),[showHost,setShowHost]=useState(false),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 async function upload(file:File){setError('');if(file.size>5*1024*1024){setError('Files must be 5 MB or smaller');return}setBusy(true);try{const base64=await new Promise<string>((resolve,reject)=>{const r=new FileReader();r.onload=()=>resolve(String(r.result).split(',')[1]);r.onerror=()=>reject(Error('Cannot read file'));r.readAsDataURL(file)});onAttach({id:crypto.randomUUID(),name:file.name,mime_type:file.type||'application/octet-stream',size_bytes:file.size,data_base64:base64});}catch(e:any){setError(e.message)}finally{setBusy(false);if(fileInput.current)fileInput.current.value=''}}
 async function fromHost(e:React.FormEvent){e.preventDefault();setError('');setBusy(true);try{const r=await apiRequest<ChatAttachment>('/api/v1/artifacts/from-file',{method:'POST',body:JSON.stringify({path:hostPath,machine:machine||null})});onAttach(r.data);setHostPath('');setShowHost(false)}catch(e:any){setError(e.message)}finally{setBusy(false)}}
 return <div className="attachment-picker">
 <input ref={fileInput} type="file" hidden onChange={e=>{const file=e.target.files?.[0];if(file)upload(file)}}/>
 <button type="button" className="icon-button" disabled={disabled||busy} onClick={()=>fileInput.current?.click()}><FiPaperclip size={17}/><span>{busy?'Reading…':'Attach file'}</span></button>
 <button type="button" className="icon-button" disabled={disabled||busy} onClick={()=>setShowHost(v=>!v)}>Host file</button>
 {showHost&&<form className="host-attachment-form" onSubmit={fromHost}><MachinePicker machine={machine} onChange={setMachine}/><label>File path<input className="chat-input-field" value={hostPath} onChange={e=>setHostPath(e.target.value)} required placeholder="/path/to/file"/></label><button className="icon-button" disabled={busy||!hostPath.trim()} type="submit">Attach from host</button></form>}
 {error&&<p role="alert">{error}</p>}
 </div>
}
export function downloadAttachment(attachment:ChatAttachment){if(!attachment.data_base64)return;const bytes=Uint8Array.from(atob(attachment.data_base64),c=>c.charCodeAt(0));const url=URL.createObjectURL(new Blob([bytes],{type:attachment.mime_type}));const a=document.createElement('a');a.href=url;a.download=attachment.name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
