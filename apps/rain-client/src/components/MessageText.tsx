export function MessageText({text}:{text:string}){
 return <>{text.split(/(https?:\/\/[^\s<>"\]]+)/g).map((part,i)=>/^https?:\/\//.test(part)?<a key={i} href={part} target="_blank" rel="noopener noreferrer">{part}</a>:part)}</>;
}
