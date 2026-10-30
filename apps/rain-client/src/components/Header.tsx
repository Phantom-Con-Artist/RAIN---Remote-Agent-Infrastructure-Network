import {useState,useEffect,useRef} from 'react';
import {FiSun,FiMoon,FiArrowUpRight,FiX} from 'react-icons/fi';
import type {ThemeMode,NetworkMeshStatus,TabType} from '../types';
import {RainIdentity} from './RainIdentity';
const links:{id:TabType;label:string;hint:string}[]=[{id:'home',label:'Home',hint:'Your starting point'},{id:'chats',label:'Conversations',hint:'Direct chats & channels'},{id:'machines',label:'Machines',hint:'Launch & manage agents'},{id:'files',label:'Files',hint:'Browse & inspect'},{id:'mesh',label:'Network',hint:'Hubs & dataflow'},{id:'guide',label:'Guide',hint:'Manual & documentation'}];
export interface HeaderProps {theme:ThemeMode;onToggleTheme:()=>void;connectionStatus:'online'|'offline'|'connecting';latencyMs:number;meshStatus:NetworkMeshStatus|null;onOpenPairing:()=>void;title?:string;currentTab:TabType;onNavigate:(tab:TabType)=>void}
export function Header({theme,onToggleTheme,connectionStatus,latencyMs,onOpenPairing,currentTab,onNavigate}:HeaderProps){
 const [open,setOpen]=useState(false);const menu=useRef<HTMLDivElement>(null),trigger=useRef<HTMLButtonElement>(null);
 useEffect(()=>{if(!open)return;menu.current?.querySelector<HTMLButtonElement>('button')?.focus();const key=(e:KeyboardEvent)=>{if(e.key==='Escape'){setOpen(false);trigger.current?.focus()}if(e.key==='Tab'){const focusable=Array.from(menu.current?.querySelectorAll<HTMLButtonElement>('button,a[href]')||[]);const first=focusable[0],last=focusable.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus()}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus()}}};document.addEventListener('keydown',key);return()=>document.removeEventListener('keydown',key)},[open]);
 const go=(id:TabType)=>{setOpen(false);onNavigate(id)};
 return <><a className="skip-link" href="#main-content" onClick={e=>{e.preventDefault();document.getElementById('main-content')?.focus()}}>Skip to content</a><header className="app-header">
 <button type="button" className="brand-home" onClick={()=>go('home')} aria-label="RAIN home"><RainIdentity/></button>
 <nav className="desktop-navigation" aria-label="Main navigation">{links.filter(l=>l.id!=='guide').map(link=><button type="button" key={link.id} aria-current={currentTab===link.id?'page':undefined} onClick={()=>go(link.id)}>{link.label}</button>)}</nav>
 <button type="button" className="menu-trigger" aria-label="Open navigation menu" aria-expanded={open} aria-controls="workspace-menu" ref={trigger} onClick={()=>setOpen(true)}>Menu <span className="menu-lines" aria-hidden="true"><i/><i/></span></button>
 </header>
 {open&&<div className="workspace-menu" id="workspace-menu" role="dialog" aria-modal="true" aria-label="Workspace navigation" ref={menu}><div className="menu-top"><button className="brand-home" onClick={()=>go('home')} aria-label="RAIN home"><RainIdentity/></button><button className="menu-trigger" onClick={()=>{setOpen(false);trigger.current?.focus()}}>Close <FiX size={18}/></button></div>
 <nav className="menu-destinations" aria-label="Workspace pages">{links.map(link=><button type="button" key={link.id} aria-current={currentTab===link.id?'page':undefined} onClick={()=>go(link.id)}><span>{link.label}</span><FiArrowUpRight aria-hidden="true"/><small>{link.hint}</small></button>)}</nav>
 <div className="menu-utilities"><div><span className={`connection-indicator ${connectionStatus}`}/>{connectionStatus==='online'?`Connected · ${latencyMs} ms`:connectionStatus==='connecting'?'Connecting to host…':'Host unavailable'}</div><button onClick={()=>{setOpen(false);onOpenPairing()}}>Connect a device <FiArrowUpRight/></button><button onClick={onToggleTheme}>{theme==='dark'?<FiSun/>:<FiMoon/>} Use {theme==='dark'?'light':'dark'} theme</button></div>
 </div>}
 </>
}
