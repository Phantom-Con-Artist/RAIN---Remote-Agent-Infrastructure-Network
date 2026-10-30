import { lazy, Suspense } from 'react';
import { createPortal } from 'react-dom';
import { useDialogFocus } from './useDialogFocus';
const TerminalSurface = lazy(() => import('./TerminalSurface'));
export function TerminalPanel(props: {id: string;onClose?: () => void}) {
 const root = useDialogFocus(!!props.onClose, props.onClose || (() => {}));
 const terminal = <Suspense fallback={<p role="status" className="min-h-[440px] p-6">Loading terminal…</p>}><TerminalSurface {...props}/></Suspense>;
 if (!props.onClose) return terminal;
 return createPortal(<div ref={root} role="dialog" aria-modal="true" aria-label="Remote terminal" className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-md"><div className="max-h-[90dvh] w-full max-w-5xl overflow-y-auto rounded-2xl border border-border-subtle bg-canvas-surface/95 p-4 shadow-2xl backdrop-blur-xl">{terminal}</div></div>,document.body);
}
