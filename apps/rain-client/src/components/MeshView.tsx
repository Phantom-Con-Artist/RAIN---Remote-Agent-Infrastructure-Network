import React,{useEffect,useState} from 'react';
import {FiRefreshCw,FiLink} from 'react-icons/fi';
import {HubTopologyStatus,NetworkMeshStatus} from '../types';
import {apiRequest,getHostUrl,setHostUrl} from '../api';
interface MeshViewProps {topology:HubTopologyStatus|null;meshStatus:NetworkMeshStatus|null;latencyMs:number;onOpenPairing:()=>void;onRevokeDevice:(id:string)=>Promise<void>;onRefreshTopology:()=>Promise<void>;onHostUrlChanged:()=>void}
export function MeshView({topology,onOpenPairing,onRevokeDevice,onRefreshTopology,onHostUrlChanged}:MeshViewProps){
 const [host,setHost]=useState(getHostUrl()),[notice,setNotice]=useState(''),[aliases,setAliases]=useState<string[]>([]);
 useEffect(()=>{apiRequest<string[]>('/api/v1/agents/machines').then(r=>setAliases(r.data)).catch(e=>setNotice(e.message));},[]);
 function save(e:React.FormEvent){e.preventDefault();try{const url=new URL(host);if(!['http:','https:'].includes(url.protocol))throw Error('Use an HTTP or HTTPS host address');setHostUrl(url.toString().replace(/\/$/,''));onHostUrlChanged();setNotice('Host address saved.')}catch(e:any){setNotice(e.message)}}
 return <div className="mesh-screen connections-screen">
 <h2 className="section-title">Connections</h2>
 <section><h3>RAIN host</h3><p>The client connects to your RAIN daemon. For remote access, use the local address of your SSH tunnel.</p>
 <form onSubmit={save}><label htmlFor="host-address">Host address</label><div className="connection-address"><input id="host-address" className="chat-input-field" value={host} onChange={e=>setHost(e.target.value)} required/><button className="icon-button primary" type="submit">Save address</button></div></form></section>
 <section><h3>Execution machines</h3><p>SSH machines configured on the host are available in the Machines and Files selectors.</p><div className="connection-machine-row"><strong>This host</strong><span>{topology?.hub_name||'Waiting for host'}</span></div>{aliases.map(alias=><div className="connection-machine-row" key={alias}><strong>{alias}</strong><span>SSH</span></div>)}{!aliases.length&&<p>No SSH machines configured. Add machine aliases on the host to make them available here.</p>}</section>
 <section><div className="section-header-row"><h3>Paired devices</h3><div className="header-actions"><button className="icon-button" onClick={onRefreshTopology} aria-label="Refresh paired devices"><FiRefreshCw/></button><button className="icon-button" onClick={onOpenPairing}><FiLink/>Pair device</button></div></div>
 {topology?.spokes?.map(device=><div className="connection-machine-row" key={device.device_id}><div><strong>{device.device_name}</strong><small>{device.platform} · {device.role}</small></div><span>{device.is_online?'Recently seen':'Offline'}</span><button className="icon-button" onClick={()=>onRevokeDevice(device.device_id).catch(e=>setNotice(e.message))}>Revoke</button></div>)}{!topology?.spokes?.length&&<p>No paired devices.</p>}</section>
 {notice&&<p role="status">{notice}</p>}
 </div>
}
