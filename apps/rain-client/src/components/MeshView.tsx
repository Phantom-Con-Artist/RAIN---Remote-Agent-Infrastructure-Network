import React,{useEffect,useState} from 'react';
import {FiRefreshCw,FiLink} from 'react-icons/fi';
import {HubTopologyStatus,NetworkMeshStatus} from '../types';
import {apiRequest,getHostUrl,setHostUrl,normalizeHostUrl} from '../api';
interface MeshViewProps {topology:HubTopologyStatus|null;meshStatus:NetworkMeshStatus|null;latencyMs:number;onOpenPairing:()=>void;onRevokeDevice:(id:string)=>Promise<void>;onRefreshTopology:()=>Promise<void>;onHostUrlChanged:()=>void}
export function MeshView({topology,onOpenPairing,onRevokeDevice,onRefreshTopology,onHostUrlChanged}:MeshViewProps){
 const [host,setHost]=useState(getHostUrl()),[notice,setNotice]=useState(''),[aliases,setAliases]=useState<string[]>([]),[checking,setChecking]=useState(false),[addressError,setAddressError]=useState('');
 useEffect(()=>{apiRequest<string[]>('/api/v1/agents/machines').then(r=>setAliases(r.data)).catch(e=>setNotice(e.message));},[]);
 async function save(e:React.FormEvent){
  e.preventDefault(); if(checking)return; setAddressError('');setNotice('');setChecking(true);
  try {
   const next=normalizeHostUrl(host);
   const response=await fetch(`${next}/health`,{credentials:'same-origin',signal:AbortSignal.timeout(6000)});
   if(response.status===401)throw Error('Open this hub address in your browser and sign in before connecting.');
   if(response.status===403)throw Error('Your hub denied access. Check your private connection and hub access settings.');
   const body=await response.json().catch(()=>null);
   if(!response.ok||!body?.success||!body?.data?.server_time)throw Error('This address is not a ready RAIN hub. Check the address and server.');
   setHostUrl(next);onHostUrlChanged();setNotice('Connected to your hub.');
  }catch(e){setAddressError(e instanceof TypeError?'Cannot reach this hub. Check its address and your private connection. You can also open the hub address directly in your browser.':e instanceof Error&&e.name==='TimeoutError'?'Connection timed out. Your current hub address is unchanged.':e instanceof Error?e.message:'Could not connect to this hub.');}
  finally{setChecking(false);}
 }
 return <div className="mesh-screen connections-screen">
 <h2 className="section-title">Connections</h2>
 <section><h3>Your server hub</h3><p>Your own computer runs RAIN, stores your work, and routes agent activity. This browser or phone is a remote client. Closing it does not stop work on the hub.</p><p>Connect through your private network using the hub’s HTTPS address. Authentication and network membership are managed by the hub owner.</p>
 <form onSubmit={save}><label htmlFor="host-address">Hub address</label><div className="connection-address"><input id="host-address" className="chat-input-field" type="text" inputMode="url" autoCapitalize="none" autoCorrect="off" spellCheck={false} aria-describedby="hub-address-help hub-address-error" aria-invalid={!!addressError} value={host} onChange={e=>{setHost(e.target.value);setAddressError('')}} required/><button className="icon-button primary" type="submit" disabled={checking}>{checking?'Checking…':'Connect to hub'}</button></div><p id="hub-address-help">Use HTTPS for remote access. A localhost address refers to the device you are using.</p><p id="hub-address-error" role="alert">{addressError}</p></form><p role="status">{notice}</p></section>
 <section><h3>Execution machines</h3><p>Agents run on the server hub. Additional SSH execution machines are optional and are managed on that server.</p><div className="connection-machine-row"><strong>Server hub</strong><span>{topology?.hub_name||'Hub information unavailable'}</span></div>{aliases.map(alias=><div className="connection-machine-row" key={alias}><strong>{alias}</strong><span>SSH</span></div>)}{!aliases.length&&<p>No additional execution machines configured. Your phone does not need SSH or a worker agent.</p>}</section>
 <section><div className="section-header-row"><h3>Worker enrollment</h3><div className="header-actions"><button className="icon-button" onClick={onRefreshTopology} aria-label="Refresh paired devices"><FiRefreshCw/></button><button className="icon-button" onClick={onOpenPairing}><FiLink/>Enroll worker</button></div></div>
 {topology?.spokes?.map(device=><div className="connection-machine-row" key={device.device_id}><div><strong>{device.device_name}</strong><small>{device.platform} · {device.role}</small></div><span>{device.is_online?'Recently seen':'Offline'}</span><button className="icon-button" onClick={()=>onRevokeDevice(device.device_id).catch(e=>setNotice(e.message))}>Revoke</button></div>)}{!topology?.spokes?.length&&<p>No enrolled workers. Remote clients connect through the private gateway; worker enrollment does not grant private-network access.</p>}</section>

 </div>
}
