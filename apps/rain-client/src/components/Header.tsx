import { useEffect, useRef, useState } from 'react';
import { FiSun, FiMoon, FiX, FiMenu, FiPlus, FiServer, FiMessageSquare, FiCpu, FiFolder, FiShare2, FiBookOpen, FiHome } from 'react-icons/fi';
import type { ThemeMode, NetworkMeshStatus, TabType } from '../types';
import { getHostUrl } from '../api';
import { RainIdentity } from './RainIdentity';
const links = [{ id: 'home', label: 'Home', Icon: FiHome }, { id: 'chats', label: 'Conversations', Icon: FiMessageSquare }, { id: 'machines', label: 'Machines', Icon: FiCpu }, { id: 'files', label: 'Files', Icon: FiFolder }, { id: 'mesh', label: 'Network', Icon: FiShare2 }, { id: 'guide', label: 'Guide', Icon: FiBookOpen }] as const;
export interface HeaderProps {
    theme: ThemeMode;
    onToggleTheme: () => void;
    connectionStatus: 'online' | 'offline' | 'connecting';
    latencyMs: number;
    meshStatus: NetworkMeshStatus | null;
    onOpenPairing: () => void;
    title?: string;
    currentTab: TabType;
    onNavigate: (tab: TabType) => void;
}
export function Header({ theme, onToggleTheme, connectionStatus, latencyMs, currentTab, onNavigate, onOpenPairing }: HeaderProps) {
    const [open, setOpen] = useState(false), menu = useRef<HTMLDivElement>(null), trigger = useRef<HTMLButtonElement>(null);
    useEffect(() => { if (!open)
        return; menu.current?.querySelector<HTMLButtonElement>('button')?.focus(); const key = (e: KeyboardEvent) => { if (e.key === 'Escape') {
        setOpen(false);
        trigger.current?.focus();
    } if (e.key === 'Tab') {
        const controls = Array.from(menu.current?.querySelectorAll<HTMLButtonElement>('button,a[href]') || []), first = controls[0], last = controls.at(-1);
        if (e.shiftKey && document.activeElement === first) {
            e.preventDefault();
            last?.focus();
        }
        else if (!e.shiftKey && document.activeElement === last) {
            e.preventDefault();
            first?.focus();
        }
    } }; document.addEventListener('keydown', key); return () => document.removeEventListener('keydown', key); }, [open]);
    const gateway = new URL(getHostUrl()), secure = gateway.protocol === 'https:', online = connectionStatus === 'online';
    function go(tab: TabType) { setOpen(false); onNavigate(tab); }
    return <><a href="#main-content" className="fixed -top-24 left-4 z-[100] rounded-lg bg-canvas-elevated p-4 focus:top-4" onClick={e => { e.preventDefault(); document.getElementById('main-content')?.focus(); }}>Skip to operations</a><header className="app-header relative z-30 flex h-16 shrink-0 items-center gap-3 border-b border-border-subtle bg-canvas-surface/85 px-4 backdrop-blur-xl lg:gap-5 lg:px-6">
 <button className="shrink-0 p-0" onClick={() => go('home')} aria-label="RAIN home"><RainIdentity /></button>
 <div className="hidden min-w-0 border-l border-border-subtle pl-4 lg:block"><div className="flex items-center gap-2 font-mono text-[10px] tracking-wider text-ink-secondary"><span className={`relative inline-flex size-2 shrink-0 rounded-full ${online ? 'bg-status-online motion-safe:animate-beacon' : connectionStatus === 'connecting' ? 'bg-status-warning' : 'bg-status-danger'}`}/><span className="max-w-44 truncate">GATEWAY: {gateway.host}</span></div><div className="mt-1 font-mono text-[9px] tracking-wider text-ink-muted">RTT: {online ? `${latencyMs}ms` : '—'} · {secure ? 'TLS ACTIVE' : 'LOCAL LOOPBACK'} · SYSADMIN</div></div>
 <nav className="ml-auto hidden items-center gap-1 rounded-xl border border-border-chisel bg-canvas-base/50 p-1 xl:flex" aria-label="Main navigation">{links.map(({ id, label, Icon }) => <button type="button" key={id} onClick={() => go(id)} aria-current={currentTab === id ? 'page' : undefined} className={`relative min-h-9 gap-2 px-3 text-xs ${currentTab === id ? 'border border-border-glow bg-canvas-elevated text-ink' : 'border border-transparent text-ink-muted hover:text-ink'}`}><Icon size={13}/>{label}</button>)}</nav>
 <div className="ml-auto flex shrink-0 items-center gap-1 xl:ml-0"><button className="size-11 border border-border-subtle bg-canvas-base/40 text-ink-secondary" onClick={onOpenPairing} aria-label="Enroll network worker" title="ENROLL_NODE"><FiPlus size={17}/></button><button className="size-11 border border-border-subtle bg-canvas-base/40 text-ink-secondary" onClick={onToggleTheme} aria-label={`Use ${theme === 'dark' ? 'light' : 'dark'} theme`}>{theme === 'dark' ? <FiSun size={17}/> : <FiMoon size={17}/>}</button><button ref={trigger} className="size-11 border border-border-subtle bg-canvas-base/40 text-ink-secondary xl:hidden" aria-label="Open navigation menu" aria-expanded={open} aria-controls="workspace-menu" onClick={() => setOpen(true)}><FiMenu size={18}/></button></div>
 </header>
 {open && <div className="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-md" onClick={e => { if (e.target === e.currentTarget)
        setOpen(false); }}><div ref={menu} id="workspace-menu" className="flex h-full w-full max-w-sm flex-col border-l border-border-subtle bg-canvas-surface/95 p-4 backdrop-blur-xl" role="dialog" aria-modal="true" aria-label="Workspace navigation"><div className="flex h-16 items-center justify-between"><RainIdentity /><button className="size-11 border border-border-subtle" onClick={() => { setOpen(false); trigger.current?.focus(); }} aria-label="Close navigation menu"><FiX /></button></div><div className="my-6 flex items-center gap-3 rounded-xl border border-border-subtle bg-canvas-base/60 p-4 font-mono text-xs"><FiServer /><span className="min-w-0 truncate">{gateway.host}<small className="block text-[10px]">{connectionStatus.toUpperCase()} · {secure ? 'TLS ACTIVE' : 'LOOPBACK'}</small></span></div><nav className="space-y-2" aria-label="Workspace pages">{links.map(({ id, label, Icon }) => <button type="button" key={id} onClick={() => go(id)} aria-current={currentTab === id ? 'page' : undefined} className={`flex min-h-12 w-full justify-start gap-4 border px-4 text-sm ${currentTab === id ? 'border-border-glow bg-canvas-elevated text-status-working' : 'border-transparent text-ink-secondary'}`}><Icon />{label}</button>)}</nav><p className="mt-auto border-t border-border-subtle pt-5 font-mono text-[10px] tracking-wider text-ink-muted">REMOTE AGENT INFRASTRUCTURE NETWORK<br />KEY-AUTHENTICATED OPERATIONS CONSOLE</p></div></div>}
 </>;
}
