import { useEffect, useMemo, useRef, useState } from "react";
import { FiMinus, FiPlus, FiMaximize, FiActivity } from "react-icons/fi";
import { formatBytes } from "../api";
interface Worker {
    id: string;
    name: string;
    online: boolean;
    revoked: boolean;
    enabled: boolean;
    programs?: string[];
    resources: {
        disk_free_bytes?: number;
        cpu_cores?: number;
        ram_available_bytes?: number;
    };
}
interface Task {
    id: string;
    title: string;
    kind: string;
    status: string;
    target: string;
    worker_id?: string;
    outputs: string[];
    agent_id?: string;
    request_id?: string;
}
export interface SecuritySnapshot {
    agents: {
        id: string;
        name: string;
        revoked: boolean;
        worker?: string;
        task_id?: string;
    }[];
    groups: {
        id: string;
        name: string;
        members: string[];
        files: string[];
        message_count?: number;
        folders?: {
            id: string;
            worker: string;
            path: string;
        }[];
        messages: {
            id: string;
            sender: string;
            text: string;
            created_at: number;
        }[];
    }[];
    guests: {
        id: string;
        agent: string;
        worker: string;
        path: string;
        reason: string;
        status: string;
        expires_at: number;
        group?: string;
    }[];
}
interface Node {
    id: string;
    name: string;
    role: string;
    state: string;
    x: number;
    y: number;
    worker?: Worker;
    agentId?: string;
    taskId?: string;
    group?: SecuritySnapshot["groups"][number];
}
export function AdminNetworkCanvas({ hub, workers, tasks, security, stale, onControl, onAgentRevoke, onTaskCancel, }: {
    hub?: {
        id: string;
        name: string;
    };
    workers: Worker[];
    tasks: Task[];
    security: SecuritySnapshot;
    stale: boolean;
    onControl: (worker: string, action: string) => void;
    onAgentRevoke: (id: string) => void;
    onTaskCancel: (id: string) => void;
}) {
    const host = useRef<HTMLDivElement>(null), drag = useRef<{
        id: number;
        x: number;
        y: number;
        ox: number;
        oy: number;
    } | null>(null);
    const [pose, setPose] = useState({ x: 0, y: 0, scale: 1 }), [selected, setSelected] = useState("hub"), [motion, setMotion] = useState(true), [visible, setVisible] = useState(true);
    useEffect(() => {
        const el = host.current;
        if (!el)
            return;
        const io = new IntersectionObserver(([entry]) => setVisible(entry.isIntersecting));
        io.observe(el);
        const change = () => setVisible(!document.hidden);
        document.addEventListener("visibilitychange", change);
        return () => {
            io.disconnect();
            document.removeEventListener("visibilitychange", change);
        };
    }, []);
    const model = useMemo(() => {
        const nodes: Node[] = [
            {
                id: "hub",
                name: hub?.name || "Your hub",
                role: "Server hub",
                state: stale ? "Stale data" : "Connected",
                x: 100,
                y: 90,
            },
        ];
        const edges: {
            from: string;
            to: string;
            role: string;
        }[] = [];
        let cursor = 70;
        workers.forEach((w) => {
            const y = cursor, agents = security.agents.filter((a) => a.worker === w.id);
            cursor += Math.max(130, agents.length * 84 + 40);
            nodes.push({
                id: w.id,
                name: w.name,
                role: "Worker",
                state: w.revoked
                    ? "Revoked"
                    : !w.online
                        ? "Offline"
                        : w.enabled
                            ? "Ready"
                            : "Paused",
                x: 390,
                y,
                worker: w,
            });
            edges.push({ from: "hub", to: w.id, role: "worker" });
            agents.forEach((a, j) => {
                const id = "process:" + a.id;
                nodes.push({
                    id,
                    name: a.name,
                    role: "Managed agent",
                    state: a.revoked
                        ? "Revoked"
                        : tasks.find((t) => t.id === a.task_id)?.status || "Not started",
                    x: 750,
                    y: y + j * 84,
                    agentId: a.id,
                    taskId: a.task_id,
                });
                edges.push({ from: w.id, to: id, role: "agent" });
            });
        });
        cursor = Math.max(cursor + 30, 200);
        security.groups.forEach((g) => {
            const y = cursor;
            cursor += Math.max(160, g.members.length * 84 + 40);
            nodes.push({
                id: g.id,
                name: g.name,
                role: "Protected group",
                state: g.files.length + " read-only files",
                x: 390,
                y,
                group: g,
            });
            edges.push({ from: "hub", to: g.id, role: "group" });
            g.members.forEach((id, j) => {
                const a = security.agents.find((a) => a.id === id);
                if (!a)
                    return;
                const nodeId = g.id + ":" + id;
                nodes.push({
                    id: nodeId,
                    name: a.name,
                    role: "Agent",
                    state: a.revoked ? "Revoked" : "Group member",
                    x: 750,
                    y: y + j * 84,
                    agentId: a.id,
                    taskId: a.task_id,
                });
                edges.push({ from: g.id, to: nodeId, role: "member" });
            });
        });
        security.agents
            .filter((a) => !a.worker && !security.groups.some((g) => g.members.includes(a.id)))
            .forEach((a) => {
            const id = "credential:" + a.id;
            nodes.push({
                id,
                name: a.name,
                role: "Agent credential",
                state: a.revoked ? "Revoked" : "No group assigned",
                x: 390,
                y: cursor,
                agentId: a.id,
            });
            edges.push({ from: "hub", to: id, role: "credential" });
            cursor += 100;
        });
        return {
            nodes,
            edges,
            width: 1100,
            height: Math.max(360, ...nodes.map((n) => n.y + 90)),
        };
    }, [hub, workers, security, stale, tasks]);
    const chosen = model.nodes.find((n) => n.id === selected) || model.nodes[0];
    const current = chosen.group
        ? tasks.filter((t) => (t.agent_id && chosen.group!.members.includes(t.agent_id)) ||
            security.guests.some((g) => g.group === chosen.id && t.request_id?.endsWith(":" + g.id)))
        : chosen.agentId
            ? tasks.filter((t) => t.id === chosen.taskId)
            : chosen.worker
                ? tasks.filter((t) => t.worker_id === chosen.id || t.target === chosen.id)
                : tasks;
    const running = tasks.filter((t) => t.status === "running");
    const moving = motion && visible && !stale;
    function zoom(factor: number) {
        setPose((p) => ({
            ...p,
            scale: Math.min(2, Math.max(0.4, p.scale * factor)),
        }));
    }
    function reset() {
        const width = host.current?.clientWidth || model.width;
        setPose({
            x: 12,
            y: 12,
            scale: Math.min(1, (width - 24) / model.width, 600 / model.height),
        });
    }
    const fitted = useRef(false);
    useEffect(() => {
        if (!fitted.current && workers.length) {
            fitted.current = true;
            reset();
        }
    }, [workers.length]);
    return (<section className="admin-network relative min-w-0 overflow-hidden" aria-label="Interactive network dashboard">
      <div className="cloud-section-heading mb-4 flex flex-wrap items-center justify-between gap-3 [&>span]:font-mono [&>span]:text-xs [&>span]:text-ink-muted">
        <div>
          <h3>Network control</h3>
          <p>
            {stale
            ? "Connection interrupted. Animation paused."
            : `${running.length} active task${running.length === 1 ? "" : "s"} · ${workers.filter((w) => w.online && !w.revoked).length} workers online`}
          </p>
        </div>
        <div className="admin-canvas-tools flex flex-wrap gap-2">
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" aria-label="Zoom out network" onClick={() => zoom(0.8)}>
            <FiMinus />
          </button>
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" aria-label="Zoom in network" onClick={() => zoom(1.25)}>
            <FiPlus />
          </button>
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" aria-label="Reset network view" onClick={reset}>
            <FiMaximize />
          </button>
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" aria-pressed={motion} onClick={() => setMotion((v) => !v)}>
            <FiActivity />
            {motion ? "Pause motion" : "Enable motion"}
          </button>
        </div>
      </div>
      <div className="admin-network-layout relative grid min-w-0 gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div className="admin-canvas relative min-w-0 max-h-[620px] overflow-auto rounded-xl border border-border-chisel bg-canvas-base/50 touch-none cursor-grab active:cursor-grabbing" ref={host} tabIndex={0} role="region" aria-label="Network tree canvas. Drag the background to pan. Use plus, minus or Home to adjust the view." onKeyDown={(e) => {
            if (e.target !== e.currentTarget)
                return;
            if (e.key === "+" || e.key === "=")
                zoom(1.25);
            if (e.key === "-")
                zoom(0.8);
            if (e.key === "Home") {
                e.preventDefault();
                reset();
            }
            if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
                e.preventDefault();
                setPose((p) => ({
                    ...p,
                    x: p.x +
                        (e.key === "ArrowLeft"
                            ? 30
                            : e.key === "ArrowRight"
                                ? -30
                                : 0),
                    y: p.y +
                        (e.key === "ArrowUp" ? 30 : e.key === "ArrowDown" ? -30 : 0),
                }));
            }
        }} onPointerDown={(e) => {
            if ((e.target as Element).closest("[data-network-node]") ||
                drag.current)
                return;
            drag.current = {
                id: e.pointerId,
                x: e.clientX,
                y: e.clientY,
                ox: pose.x,
                oy: pose.y,
            };
            e.currentTarget.setPointerCapture(e.pointerId);
        }} onPointerMove={(e) => {
            const d = drag.current;
            if (d?.id === e.pointerId)
                setPose((p) => ({
                    ...p,
                    x: d.ox + e.clientX - d.x,
                    y: d.oy + e.clientY - d.y,
                }));
        }} onPointerUp={(e) => {
            if (drag.current?.id === e.pointerId) {
                drag.current = null;
                if (e.currentTarget.hasPointerCapture(e.pointerId))
                    e.currentTarget.releasePointerCapture(e.pointerId);
            }
        }} onPointerCancel={() => {
            drag.current = null;
        }} onLostPointerCapture={() => {
            drag.current = null;
        }} onBlur={() => {
            drag.current = null;
        }}>
          <svg viewBox={`0 0 ${model.width} ${Math.min(model.height, 650)}`} style={{
            minWidth: Math.min(model.width, 680),
            height: Math.min(model.height, 650)
        }} role="group" aria-label="Hub, workers, protected groups and agents">
            <g transform={`translate(${pose.x},${pose.y}) scale(${pose.scale})`}>
              {model.edges.map((edge) => {
            const a = model.nodes.find((n) => n.id === edge.from)!, b = model.nodes.find((n) => n.id === edge.to)!;
            const task = running.find((t) => (t.worker_id || t.target) === b.id || t.id === b.taskId);
            const message = edge.role === "member"
                ? security.groups
                    .find((g) => g.id === edge.from)
                    ?.messages.slice(-1)
                    .find((m) => edge.to.endsWith(":" + m.sender) &&
                    Date.now() / 1000 - m.created_at < 8)
                : undefined;
            const inspection = task && ["list_files", "fetch_file", "put_file"].includes(task.kind);
            return (<g key={edge.from + edge.to}>
                    <path className="admin-network-edge fill-none stroke-border-subtle stroke-1" d={`M${a.x + 205} ${a.y + 10} C${a.x + 245} ${a.y + 10},${b.x - 60} ${b.y + 10},${b.x - 20} ${b.y + 10}`}/>
                    {(task || message) && (<path className={`admin-network-flow fill-none stroke-status-working stroke-2 [stroke-dasharray:6_12] [&.inspection]:stroke-status-online [&.is-active]:motion-safe:animate-conduit [&.inspection.is-active]:[animation-direction:reverse] motion-reduce:animate-none ${inspection ? "inspection" : ""} ${moving ? "is-active" : ""}`} d={`M${a.x + 205} ${a.y + 10} C${a.x + 245} ${a.y + 10},${b.x - 60} ${b.y + 10},${b.x - 20} ${b.y + 10}`}>
                        <title>
                          {task
                        ? task.title +
                            " · " +
                            (inspection
                                ? "reading shared data"
                                : task.kind === "put_file"
                                    ? "sending file"
                                    : "executing task")
                        : "Recent protected group message"}
                        </title>
                      </path>)}
                  </g>);
        })}
              {model.nodes.map((node) => (<g data-network-node="true" key={node.id} transform={`translate(${node.x - 20},${node.y - 30})`} role="button" tabIndex={0} aria-label={`${node.name}, ${node.role}, ${node.state}`} aria-pressed={chosen.id === node.id} onClick={() => setSelected(node.id)} onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    setSelected(node.id);
                }
            }} className={`admin-network-node cursor-pointer [&_rect]:fill-canvas-surface [&_rect]:stroke-border-subtle [&_rect]:stroke-1 [&_text]:fill-ink [&_text]:font-sans [&_text]:text-[13px] [&_circle]:fill-status-online [&.unavailable_circle]:fill-status-danger [&.selected_rect]:stroke-status-working [&.selected_rect]:stroke-2 [&.selected_rect]:fill-canvas-elevated focus-visible:[&_rect]:stroke-status-working focus-visible:[&_rect]:stroke-2 ${chosen.id === node.id ? "selected" : ""} ${node.state === "Revoked" || node.state === "Offline" || node.state === "Stale data" ? "unavailable" : ""}`}>
                  <rect width="225" height="80" rx="8"/>
                  <circle cx="15" cy="20" r="3" style={{fill: node.state === "Revoked" ? "var(--rain-danger)" : node.state === "Paused" ? "var(--rain-warning)" : ["Offline", "Stale data", "Not started"].includes(node.state) ? "var(--rain-offline)" : "var(--rain-online)"}}/>
                  <text x="28" y="24">
                    {node.name.length > 24
                ? node.name.slice(0, 22) + "…"
                : node.name}
                  </text>
                  <text className="node-detail fill-ink-muted! font-mono! text-[10px]!" x="15" y="49">
                    {node.role} · {node.state}
                  </text>
                  {node.worker && <text className="fill-ink-secondary! font-mono! text-[9px]!" x="15" y="66">{node.worker.resources.cpu_cores ?? '—'}C · {node.worker.resources.ram_available_bytes == null ? 'RAM —' : formatBytes(node.worker.resources.ram_available_bytes)+' FREE'} · {running.filter(t=>(t.worker_id||t.target)===node.id).length} ACTIVE</text>}
                </g>))}
            </g>
          </svg>
        </div>
        <aside className="admin-node-detail glass-panel min-w-0 rounded-2xl p-5 shadow-2xl xl:sticky xl:top-4 xl:max-h-[620px] xl:overflow-y-auto [&_h4]:mb-3 [&_h4]:break-all [&_p]:text-xs [&_dl]:my-4 [&_dl]:grid [&_dl]:grid-cols-2 [&_dl]:gap-3 [&_dl]:font-mono [&_dl]:text-xs [&_ul]:space-y-3 [&_li]:border-b [&_li]:border-border-chisel [&_li]:pb-3 [&_li]:text-xs [&_li_span]:block [&_li_span]:mt-1 [&_li_span]:font-mono [&_li_span]:text-ink-muted" aria-label="Selected network element">
          <h4>{chosen.name}</h4>
          <p>
            {chosen.role} · {chosen.state}
          </p>
          <div className="my-4 space-y-2 rounded-lg border border-border-chisel bg-canvas-base/50 p-3"><span className="telemetry-label">NODE_ID</span><code className="block break-all text-[10px]">{chosen.id === 'hub' ? (hub?.id || 'NOT_REPORTED') : chosen.id}</code><button className="min-h-11 w-full border border-border-subtle text-xs" onClick={async()=>{try{await navigator.clipboard.writeText(chosen.id === 'hub' ? (hub?.id || 'NOT_REPORTED') : chosen.id)}catch{window.alert('Clipboard unavailable. Select the node ID to copy.')}}}>COPY_NODE_ID</button><p className="text-ink-muted">Key fingerprint: NOT_REPORTED</p></div>
          {chosen.worker && (<>
              <dl>
                <dt>CPU cores</dt>
                <dd>{chosen.worker.resources.cpu_cores ?? "Not reported"}</dd>
                <dt>Available memory</dt>
                <dd>
                  {chosen.worker.resources.ram_available_bytes == null
                ? "Not reported"
                : formatBytes(chosen.worker.resources.ram_available_bytes)}
                </dd>
              </dl>
              <div className="my-4 flex flex-wrap gap-2">{chosen.worker.programs?.map(program=><code key={program} className="rounded border border-border-subtle bg-canvas-elevated px-2 py-1 text-[10px]">{program}</code>)}</div>
              {!chosen.worker.revoked && (<div className="admin-node-controls my-4 flex flex-wrap gap-2">
                  <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => onControl(chosen.id, chosen.worker!.enabled ? "pause" : "resume")}>
                    {chosen.worker.enabled ? "PAUSE_SHARING" : "RESUME_SHARING"}
                  </button>
                  <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => {
                    if (window.confirm(`Revoke ${chosen.name} and cancel its active work?`))
                        onControl(chosen.id, "revoke");
                }}>
                    REVOKE_WORKER_KEY
                  </button>
                </div>)}
            </>)}
          {chosen.agentId && (<div className="admin-node-controls my-4 flex flex-wrap gap-2">
              {chosen.taskId && (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => onTaskCancel(chosen.taskId!)}>
                  TERMINATE_AGENT
                </button>)}
              <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => {
                if (window.confirm(`Revoke ${chosen.name} and its guest access?`))
                    onAgentRevoke(chosen.agentId!);
            }}>
                REVOKE_AGENT_KEY
              </button>
            </div>)}
          {chosen.group && (<p>
              {chosen.group.members.length} members ·{" "}
              {chosen.group.files.length} explicitly shared, read-only files.
              Manage membership and grants in Permissions below.
            </p>)}
          <h4>Recent work</h4>
          {current.length ? (<ul>
              {current.slice(-5).map((t) => (<li key={t.id}>
                  <strong>{t.title}</strong>
                  <span>
                    {t.status} · {t.kind.replaceAll("_", " ")}
                  </span>
                </li>))}
            </ul>) : (<p>No recorded tasks for this element.</p>)}
        </aside>
      </div>
      <p className="admin-canvas-caption mt-4 font-mono text-[10px] text-ink-muted">
        Select a node to inspect and control it. Moving paths show running tasks
        reported by the hub; they do not claim packet-level transfer progress.
      </p>
    </section>);
}
