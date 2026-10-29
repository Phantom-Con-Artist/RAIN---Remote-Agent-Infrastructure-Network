import {useEffect,useRef} from 'react';
export function useDialogFocus(open:boolean,onClose:()=>void){
 const root=useRef<HTMLDivElement>(null),close=useRef(onClose);close.current=onClose;
 useEffect(()=>{
  if(!open)return;const previous=document.activeElement as HTMLElement|null;
  const timer=setTimeout(()=>root.current?.querySelector<HTMLElement>('input:not([disabled]),button:not([disabled])')?.focus(),0);
  const key=(e:KeyboardEvent)=>{const el=root.current;if(!el)return;if(e.key==='Escape'){e.preventDefault();close.current();return}if(e.key!=='Tab')return;const focusable=[...el.querySelectorAll<HTMLElement>('button:not([disabled]),input:not([disabled]),textarea:not([disabled]),select:not([disabled]),[tabindex="0"]')].filter(e=>e.offsetParent!==null);const first=focusable[0],last=focusable.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus()}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus()}};
  document.addEventListener('keydown',key);return()=>{clearTimeout(timer);document.removeEventListener('keydown',key);if(previous?.isConnected)previous.focus()};
 },[open]);return root;
}
