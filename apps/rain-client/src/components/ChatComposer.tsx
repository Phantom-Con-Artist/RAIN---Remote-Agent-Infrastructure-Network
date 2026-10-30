import { useState, useEffect, useRef, useLayoutEffect } from 'react';
import { apiRequest } from '../api';
import type { AgentSession } from '../types';
interface Suggestion {
    name: string;
    description: string;
    value?: string;
}
interface Catalog {
    commands: Suggestion[];
    efforts?: string[];
}
export function ChatComposer({ id, members, group, value, onChange, onSend, disabled, placeholder }: {
    id: string;
    members: AgentSession[];
    group: boolean;
    value: string;
    onChange: (v: string) => void;
    onSend: () => void;
    disabled: boolean;
    placeholder: string;
}) {
    const [catalog, setCatalog] = useState<Catalog>({ commands: [] }), [cursor, setCursor] = useState(0), [selected, setSelected] = useState(0), [dismissed, setDismissed] = useState(false);
    const input = useRef<HTMLTextAreaElement>(null);
    useEffect(() => { const controller = new AbortController(); apiRequest<Catalog>(`/api/v1/conversations/${id}/commands`, { signal: controller.signal }).then(r => setCatalog(r.data)).catch(() => { }); return () => controller.abort(); }, [id]);
    const before = value.slice(0, cursor), mention = before.match(/(?:^|\s)@([^@\s{}]*)$/), slash = value.startsWith('/') && !value.includes('\n');
    let options: Suggestion[] = [], start = 0;
    if (mention) {
        start = cursor - mention[1].length - 1;
        options = [...(group ? [{ name: '@all', description: 'Everyone in this channel', value: '@all ' }] : []), ...members.map(m => ({ name: `@${m.name}`, description: `${m.machine_name} · ${m.state}`, value: `@{${m.name}} ` }))].filter(s => s.name.toLowerCase().includes(mention[1].toLowerCase()));
    }
    else if (slash) {
        if (value.startsWith('/effort ')) {
            options = [...(catalog.efforts || []), 'default'].filter(x => x.startsWith(value.slice(8))).map(x => ({ name: x, description: 'Effort for this direct thread', value: `/effort ${x} ` }));
        }
        else if (value.startsWith('/model ')) {
            options = [{ name: 'default', description: 'Use the provider default', value: '/model default ' }].filter(x => x.name.startsWith(value.slice(7)));
        }
        else {
            options = catalog.commands.filter(s => s.name.startsWith(value.trim())).map(s => ({ ...s, value: `${s.name} ` }));
        }
    }
    options = options.slice(0, 9);
    const open = !dismissed && options.length > 0;
    const choose = (s: Suggestion) => { const v = mention ? value.slice(0, start) + (s.value || s.name) + value.slice(cursor) : s.value || s.name; onChange(v); setCursor(v.length); setDismissed(true); requestAnimationFrame(() => { input.current?.focus(); input.current?.setSelectionRange(v.length, v.length); }); };
    useLayoutEffect(() => { const el = input.current; if (el) {
        el.style.height = '44px';
        el.style.height = Math.min(160, el.scrollHeight) + 'px';
    } }, [value]);
    return <div className="composer-editor relative min-w-0 flex-1">
 {open && <div className="composer-suggestions absolute bottom-full left-0 z-30 mb-4 max-h-72 w-full min-w-56 overflow-y-auto rounded-xl border border-border-subtle bg-canvas-elevated p-2 [&>button]:flex [&>button]:w-full [&>button]:flex-col [&>button]:items-start [&>button]:text-left [&>button]:text-xs [&>button[aria-selected=true]]:bg-canvas-hover" id="chat-suggestions" role="listbox" aria-label={mention ? 'Channel members' : 'Available commands'}>{options.map((s, i) => <button type="button" role="option" aria-selected={selected % options.length === i} id={`chat-suggestion-${i}`} key={s.name} onMouseDown={e => e.preventDefault()} onClick={() => choose(s)}><strong>{s.name}</strong><span>{s.description}</span></button>)}</div>}
 <textarea ref={input} rows={1} role="combobox" aria-autocomplete="list" aria-expanded={open} aria-controls={open ? 'chat-suggestions' : undefined} aria-activedescendant={open ? `chat-suggestion-${selected % options.length}` : undefined} aria-label="Message" className="chat-input-field min-h-11 w-full min-w-0 resize-none border-0! bg-transparent! px-3 py-2 text-base text-ink placeholder:text-ink-muted" value={value} disabled={disabled} placeholder={placeholder} onSelect={e => setCursor(e.currentTarget.selectionStart)} onChange={e => { onChange(e.target.value); setCursor(e.target.selectionStart); setSelected(0); setDismissed(false); }} onKeyDown={e => {
            if (e.nativeEvent.isComposing)
                return;
            if (open && ['ArrowDown', 'ArrowUp', 'Tab', 'Enter', 'Escape'].includes(e.key) && !e.shiftKey) {
                e.preventDefault();
                if (e.key === 'Escape')
                    setDismissed(true);
                else if (e.key === 'ArrowDown')
                    setSelected(i => (i + 1) % options.length);
                else if (e.key === 'ArrowUp')
                    setSelected(i => (i + options.length - 1) % options.length);
                else
                    choose(options[selected % options.length]);
                return;
            }
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                onSend();
            }
        }}/>
 </div>;
}
