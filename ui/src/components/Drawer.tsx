import { useEffect, useId, useRef, type ReactNode } from "react";
import { Button } from "./Button";
import styles from "./Drawer.module.css";
type Props = {open:boolean;onClose:()=>void;title:string;description?:string;children:ReactNode;footer?:ReactNode;opener?:HTMLElement|null};
export function Drawer({open,onClose,title,description,children,footer,opener}:Props) {
 const dialog=useRef<HTMLDialogElement>(null);const heading=useRef<HTMLHeadingElement>(null);const returnTo=useRef<HTMLElement|null>(null);const id=useId();
 useEffect(()=>{
  const node=dialog.current;if(!node)return;
  if(open&&!node.open){returnTo.current=opener??(document.activeElement instanceof HTMLElement?document.activeElement:null);node.showModal();heading.current?.focus();}
  else if(!open&&node.open){node.close();if(returnTo.current?.isConnected)returnTo.current.focus();}
 },[open,opener]);
 useEffect(()=>()=>{if(returnTo.current?.isConnected)returnTo.current.focus();},[]);
 return <dialog ref={dialog} className={styles.drawer} aria-labelledby={`${id}-title`} aria-describedby={description?`${id}-description`:undefined} onCancel={event=>{event.preventDefault();onClose();}}>
  <header className={styles.header}><div><h2 ref={heading} tabIndex={-1} id={`${id}-title`}>{title}</h2>{description&&<p id={`${id}-description`}>{description}</p>}</div><Button onClick={onClose}>Close</Button></header>
  <div className={styles.content}>{children}</div>
  {footer&&<footer className={styles.footer}>{footer}</footer>}
 </dialog>;
}
