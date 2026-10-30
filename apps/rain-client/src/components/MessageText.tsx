import { useEffect, useState } from 'react';
import { FiCopy, FiCheck, FiCode } from 'react-icons/fi';
function tokens(code: string) { return code.split(/("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\b(?:const|let|var|function|return|import|from|export|async|await|if|else|for|while|class|def|print|pub|fn|use|struct|impl|match|true|false|null|None|True|False|sudo)\b|\b\d+(?:\.\d+)?\b|\/\/[^\n]*|#[^\n]*)/g).map((token, i) => { const color = /^(?:"|')/.test(token) ? 'text-emerald-400' : /^(?:\/\/|#)/.test(token) ? 'text-slate-400 italic' : /^\d/.test(token) ? 'text-amber-300' : /^[a-zA-Z]/.test(token) && !/[\s\W]/.test(token) ? 'text-cyan-300' : ''; return <span key={i} className={color}>{token}</span>; }); }
function CodeBlock({ language, code }: {
    language: string;
    code: string;
}) { const [copied, setCopied] = useState(false), [error, setError] = useState(''); useEffect(() => { if (!copied)
    return; const timer = setTimeout(() => setCopied(false), 2000); return () => clearTimeout(timer); }, [copied]); return <div className="my-3 max-w-full overflow-hidden rounded-xl border border-border-subtle bg-[#080a0f] text-[#f1f5f9]"><div className="flex min-h-11 items-center justify-between gap-3 border-b border-white/10 px-3 font-mono text-[10px] uppercase tracking-wider text-slate-300"><span className="flex items-center gap-2"><FiCode />{language || 'TEXT'} · {code.split('\n').length} LINES</span><button type="button" className="min-h-11 text-xs text-white" aria-label="Copy code" onClick={async () => { try {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setError('');
}
catch {
    setError('Clipboard unavailable; select the code to copy.');
} }}>{copied ? <FiCheck /> : <FiCopy />}{copied ? 'COPIED' : 'COPY_CODE'}</button></div><pre className="m-0 max-h-[440px] rounded-none border-0 bg-transparent p-4 font-mono text-xs leading-6 text-[#f1f5f9]"><code>{tokens(code)}</code></pre>{error && <p role="status" className="px-4 py-2 text-xs text-white">{error}</p>}</div>; }
function prose(text: string, key: number) { return <span key={key} className="whitespace-pre-wrap break-words">{text.split(/(https?:\/\/[^\s<>"\]]+)/g).map((part, i) => /^https?:\/\//.test(part) ? <a key={i} href={part} target="_blank" rel="noopener noreferrer" className="underline decoration-current/40 underline-offset-4">{part}</a> : part)}</span>; }
export function MessageText({ text }: {
    text: string;
}) { const parts = []; const fenced = /```([^\n`]*)\n([\s\S]*?)```/g; let last = 0, index = 0; for (const match of text.matchAll(fenced)) {
    const offset = match.index!;
    if (offset > last)
        parts.push(prose(text.slice(last, offset), index++));
    parts.push(<CodeBlock key={index++} language={match[1].trim()} code={match[2].replace(/\n$/, '')}/>);
    last = offset + match[0].length;
} if (last < text.length)
    parts.push(prose(text.slice(last), index++)); return <>{parts}</>; }
