import React, {lazy, Suspense} from 'react';
const TerminalSurface = lazy(() => import('./TerminalSurface'));
export function TerminalPanel(props: {id:string; onClose?:()=>void}) {
  return <Suspense fallback={<p role="status">Loading terminal…</p>}><TerminalSurface {...props}/></Suspense>;
}
