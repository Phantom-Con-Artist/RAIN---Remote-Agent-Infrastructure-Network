import React, {useEffect,useState} from 'react';
import {FiDownload,FiFile,FiChevronDown} from 'react-icons/fi';
import {ChatAttachment} from '../types';
import {apiRequest,formatBytes,getHostUrl,getAuthToken} from '../api';
import {downloadAttachment} from './AttachmentPicker';

export function AttachmentCard({file,initiallyOpen=false}:{file:ChatAttachment;initiallyOpen?:boolean}) {
 const [open,setOpen]=useState(initiallyOpen),[url,setUrl]=useState(''),[error,setError]=useState(''),[text,setText]=useState<string|null>(null),[hex,setHex]=useState('');
 const [source,setSource]=useState<{original_path:string;source_machine:string|null}|null>(null);
 const mime=(file.mime_type||'application/octet-stream').split(';')[0];
 const image=mime.startsWith('image/')&&mime!=='image/svg+xml';
 const video=mime.startsWith('video/'),audio=mime.startsWith('audio/'),pdf=mime==='application/pdf';
 const readable=mime.startsWith('text/')||/json|xml|javascript|yaml|toml|svg/.test(mime);
 const content=`${getHostUrl()}/api/v1/artifacts/${encodeURIComponent(file.id)}/content`;
 useEffect(()=>{
  if(!open)return;let cancelled=false;let objectUrl='';const controller=new AbortController();setError('');setText(null);setHex('');
  async function load(){
   if(file.data_base64){try{const bytes=Uint8Array.from(atob(file.data_base64),c=>c.charCodeAt(0));objectUrl=URL.createObjectURL(new Blob([bytes],{type:mime}));if(!cancelled)setUrl(objectUrl);}catch{if(!cancelled)setError('The saved file data is unavailable.');}}
   else {setUrl(content);apiRequest<{original_path:string;source_machine:string|null}>(`/api/v1/artifacts/${file.id}`).then(r=>{if(!cancelled)setSource(r.data)}).catch(()=>{});}
   if(readable||(!image&&!video&&!audio&&!pdf)){
    try{const headers:Record<string,string>={Range:'bytes=0-262143'};if(getAuthToken())headers.Authorization=`Bearer ${getAuthToken()}`;
     const response=await fetch(objectUrl||content,{headers,signal:controller.signal});if(!response.ok&&response.status!==416)throw Error('This file is unavailable on its source machine.');
     const bytes=new Uint8Array(await response.arrayBuffer());
     if(cancelled)return;
     if(readable&&!bytes.includes(0)){setText(new TextDecoder().decode(bytes));}
     else{setHex(Array.from(bytes.slice(0,256)).map((b,i)=>(i%16===0?`\n${i.toString(16).padStart(6,'0')}  `:'')+b.toString(16).padStart(2,'0')).join(' '));}
    }catch(e){if(!cancelled&&!controller.signal.aborted)setError(e instanceof Error?e.message:'Could not preview file');}
   }
  }
  load();return()=>{cancelled=true;controller.abort();if(objectUrl)URL.revokeObjectURL(objectUrl);};
 },[open,file.id,file.data_base64]);
 function unavailable(){setError('Your platform could not display this format or codec. Download it to open with a compatible app.');}
 return <details className="attachment-card" open={open} onToggle={e=>setOpen(e.currentTarget.open)}>
  <summary><FiFile aria-hidden="true"/><span><strong>{file.name}</strong><small>{formatBytes(file.size_bytes)} · {mime}</small></span><FiChevronDown aria-hidden="true"/></summary>
  {open&&<div className="attachment-inspector">
   <div className="attachment-actions">{file.data_base64?<button type="button" className="icon-button" onClick={()=>downloadAttachment(file)}><FiDownload/>Download</button>:<a className="icon-button" href={`${content}?download=1`} download={file.name}><FiDownload/>Download</a>}<span>Preview uses this device’s capabilities</span></div>
   {url&&image&&!error&&<img className="attachment-image" src={url} alt={file.name} loading="lazy" onError={unavailable}/>}
   {url&&video&&!error&&<video controls preload="metadata" src={url} onError={unavailable} aria-label={file.name}/>}
   {url&&audio&&!error&&<audio controls preload="metadata" src={url} onError={unavailable} aria-label={file.name}/>}
   {url&&pdf&&!error&&<iframe title={file.name} src={url} sandbox="" className="attachment-pdf"/>}
   {text!==null&&<><pre className="attachment-text">{text||'(Empty file)'}</pre>{file.size_bytes>262144&&<p>Showing the first 256 KB. Download the full file for more.</p>}</>}
   {hex&&<><p>No native preview for this type. You can download it or inspect its first bytes below.</p><pre className="attachment-hex">{hex}</pre></>}
   {error&&<p role="status" className="attachment-error">{error}</p>}
   <details className="attachment-provenance"><summary>File details</summary><dl><dt>SHA-256</dt><dd>{file.sha256||'Not recorded for this older file'}</dd>{source&&<><dt>Fetched from</dt><dd>{source.source_machine||'RAIN host'} · {source.original_path}</dd></>}</dl></details>
  </div>}
 </details>;
}
