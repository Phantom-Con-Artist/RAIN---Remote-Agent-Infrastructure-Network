import React from 'react';
import {FiSun,FiMoon,FiLink} from 'react-icons/fi';
import type {ThemeMode,NetworkMeshStatus} from '../types';
import {RainIdentity} from './RainIdentity';
export interface HeaderProps {theme:ThemeMode;onToggleTheme:()=>void;connectionStatus:'online'|'offline'|'connecting';latencyMs:number;meshStatus:NetworkMeshStatus|null;onOpenPairing:()=>void;title?:string}
export function Header({theme,onToggleTheme,connectionStatus,latencyMs,onOpenPairing,title}:HeaderProps){return <header className="app-header">
 <RainIdentity/>
 <div className="app-location"><strong>{title||'Workspace'}</strong><span>Remote agent workbench</span></div>
 <div className="header-actions"><span className="connection-state">{connectionStatus==='online'?`Connected · ${latencyMs} ms`:connectionStatus==='connecting'?'Connecting…':'Host unavailable'}</span><button className="icon-button" onClick={onOpenPairing} aria-label="Connect device"><FiLink size={18}/><span>Connect device</span></button><button className="icon-button theme-button" onClick={onToggleTheme} aria-label={`Use ${theme==='dark'?'light':'dark'} theme`}>{theme==='dark'?<FiSun size={19}/>:<FiMoon size={19}/>}</button></div>
 </header>}
