import { useEffect, useMemo, useRef, useState } from "react";
import { FiMinus, FiPlus, FiMaximize, FiActivity } from "react-icons/fi";
import { formatBytes } from "../api";
interface Worker {
  id: string;
  name: string;
  online: boolean;
  revoked: boolean;
  enabled: boolean;
  resources: { cpu_cores?: number; ram_available_bytes?: number };
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
    folders?: { id: string; worker: string; path: string }[];
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
export function AdminNetworkCanvas({
  hub,
  workers,
  tasks,
  security,
  stale,
  onControl,
  onAgentRevoke,
  onTaskCancel,
}: {
  hub?: { id: string; name: string };
  workers: Worker[];
  tasks: Task[];
  security: SecuritySnapshot;
  stale: boolean;
  onControl: (worker: string, action: string) => void;
  onAgentRevoke: (id: string) => void;
  onTaskCancel: (id: string) => void;
}) {
  const host = useRef<HTMLDivElement>(null),
    drag = useRef<{
      id: number;
      x: number;
      y: number;
      ox: number;
      oy: number;
    } | null>(null);
  const [pose, setPose] = useState({ x: 0, y: 0, scale: 1 }),
    [selected, setSelected] = useState("hub"),
    [motion, setMotion] = useState(true),
    [visible, setVisible] = useState(true);
  useEffect(() => {
    const el = host.current;
    if (!el) return;
    const io = new IntersectionObserver(([entry]) =>
      setVisible(entry.isIntersecting),
    );
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
    const edges: { from: string; to: string; role: string }[] = [];
    let cursor = 70;
    workers.forEach((w) => {
      const y = cursor,
        agents = security.agents.filter((a) => a.worker === w.id);
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
        if (!a) return;
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
      .filter(
        (a) =>
          !a.worker && !security.groups.some((g) => g.members.includes(a.id)),
      )
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
    ? tasks.filter(
        (t) =>
          (t.agent_id && chosen.group!.members.includes(t.agent_id)) ||
          security.guests.some(
            (g) => g.group === chosen.id && t.request_id?.endsWith(":" + g.id),
          ),
      )
    : chosen.agentId
      ? tasks.filter((t) => t.id === chosen.taskId)
      : chosen.worker
        ? tasks.filter(
            (t) => t.worker_id === chosen.id || t.target === chosen.id,
          )
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
  return (
    <section
      className="admin-network"
      aria-label="Interactive network dashboard"
    >
      <div className="cloud-section-heading">
        <div>
          <h3>Network control</h3>
          <p>
            {stale
              ? "Connection interrupted. Animation paused."
              : `${running.length} active task${running.length === 1 ? "" : "s"} · ${workers.filter((w) => w.online && !w.revoked).length} workers online`}
          </p>
        </div>
        <div className="admin-canvas-tools">
          <button
            className="icon-button"
            aria-label="Zoom out network"
            onClick={() => zoom(0.8)}
          >
            <FiMinus />
          </button>
          <button
            className="icon-button"
            aria-label="Zoom in network"
            onClick={() => zoom(1.25)}
          >
            <FiPlus />
          </button>
          <button
            className="icon-button"
            aria-label="Reset network view"
            onClick={reset}
          >
            <FiMaximize />
          </button>
          <button
            className="icon-button"
            aria-pressed={motion}
            onClick={() => setMotion((v) => !v)}
          >
            <FiActivity />
            {motion ? "Pause motion" : "Enable motion"}
          </button>
        </div>
      </div>
      <div className="admin-network-layout">
        <div
          className="admin-canvas"
          ref={host}
          tabIndex={0}
          role="region"
          aria-label="Network tree canvas. Drag the background to pan. Use plus, minus or Home to adjust the view."
          onKeyDown={(e) => {
            if (e.target !== e.currentTarget) return;
            if (e.key === "+" || e.key === "=") zoom(1.25);
            if (e.key === "-") zoom(0.8);
            if (e.key === "Home") {
              e.preventDefault();
              reset();
            }
            if (
              ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(
                e.key,
              )
            ) {
              e.preventDefault();
              setPose((p) => ({
                ...p,
                x:
                  p.x +
                  (e.key === "ArrowLeft"
                    ? 30
                    : e.key === "ArrowRight"
                      ? -30
                      : 0),
                y:
                  p.y +
                  (e.key === "ArrowUp" ? 30 : e.key === "ArrowDown" ? -30 : 0),
              }));
            }
          }}
          onPointerDown={(e) => {
            if (
              (e.target as Element).closest("[data-network-node]") ||
              drag.current
            )
              return;
            drag.current = {
              id: e.pointerId,
              x: e.clientX,
              y: e.clientY,
              ox: pose.x,
              oy: pose.y,
            };
            e.currentTarget.setPointerCapture(e.pointerId);
          }}
          onPointerMove={(e) => {
            const d = drag.current;
            if (d?.id === e.pointerId)
              setPose((p) => ({
                ...p,
                x: d.ox + e.clientX - d.x,
                y: d.oy + e.clientY - d.y,
              }));
          }}
          onPointerUp={(e) => {
            if (drag.current?.id === e.pointerId) {
              drag.current = null;
              if (e.currentTarget.hasPointerCapture(e.pointerId))
                e.currentTarget.releasePointerCapture(e.pointerId);
            }
          }}
          onPointerCancel={() => {
            drag.current = null;
          }}
          onLostPointerCapture={() => {
            drag.current = null;
          }}
          onBlur={() => {
            drag.current = null;
          }}
        >
          <svg
            viewBox={`0 0 ${model.width} ${Math.min(model.height, 650)}`}
            style={{
              minWidth: Math.min(model.width, 680),
              height: Math.min(model.height, 650),
            }}
            role="group"
            aria-label="Hub, workers, protected groups and agents"
          >
            <g
              transform={`translate(${pose.x},${pose.y}) scale(${pose.scale})`}
            >
              {model.edges.map((edge) => {
                const a = model.nodes.find((n) => n.id === edge.from)!,
                  b = model.nodes.find((n) => n.id === edge.to)!;
                const task = running.find(
                  (t) =>
                    (t.worker_id || t.target) === b.id || t.id === b.taskId,
                );
                const message =
                  edge.role === "member"
                    ? security.groups
                        .find((g) => g.id === edge.from)
                        ?.messages.slice(-1)
                        .find(
                          (m) =>
                            edge.to.endsWith(":" + m.sender) &&
                            Date.now() / 1000 - m.created_at < 8,
                        )
                    : undefined;
                const inspection =
                  task && ["list_files", "fetch_file"].includes(task.kind);
                return (
                  <g key={edge.from + edge.to}>
                    <path
                      className="admin-network-edge"
                      d={`M${a.x + 100} ${a.y} H${a.x + 140} V${b.y} H${b.x - 20}`}
                    />
                    {(task || message) && (
                      <path
                        className={`admin-network-flow ${inspection ? "inspection" : ""} ${moving ? "is-active" : ""}`}
                        d={`M${a.x + 100} ${a.y} H${a.x + 140} V${b.y} H${b.x - 20}`}
                      >
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
                      </path>
                    )}
                  </g>
                );
              })}
              {model.nodes.map((node) => (
                <g
                  data-network-node="true"
                  key={node.id}
                  transform={`translate(${node.x - 20},${node.y - 30})`}
                  role="button"
                  tabIndex={0}
                  aria-label={`${node.name}, ${node.role}, ${node.state}`}
                  aria-pressed={chosen.id === node.id}
                  onClick={() => setSelected(node.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      setSelected(node.id);
                    }
                  }}
                  className={`admin-network-node ${chosen.id === node.id ? "selected" : ""} ${node.state === "Revoked" || node.state === "Offline" || node.state === "Stale data" ? "unavailable" : ""}`}
                >
                  <rect width="225" height="72" rx="6" />
                  <circle cx="15" cy="20" r="3" />
                  <text x="28" y="24">
                    {node.name.length > 24
                      ? node.name.slice(0, 22) + "…"
                      : node.name}
                  </text>
                  <text className="node-detail" x="15" y="49">
                    {node.role} · {node.state}
                  </text>
                </g>
              ))}
            </g>
          </svg>
        </div>
        <aside
          className="admin-node-detail"
          aria-label="Selected network element"
        >
          <h4>{chosen.name}</h4>
          <p>
            {chosen.role} · {chosen.state}
          </p>
          {chosen.worker && (
            <>
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
              {!chosen.worker.revoked && (
                <div className="admin-node-controls">
                  <button
                    className="icon-button"
                    onClick={() =>
                      onControl(
                        chosen.id,
                        chosen.worker!.enabled ? "pause" : "resume",
                      )
                    }
                  >
                    {chosen.worker.enabled ? "Pause sharing" : "Resume sharing"}
                  </button>
                  <button
                    className="icon-button"
                    onClick={() => {
                      if (
                        window.confirm(
                          `Revoke ${chosen.name} and cancel its active work?`,
                        )
                      )
                        onControl(chosen.id, "revoke");
                    }}
                  >
                    Revoke worker
                  </button>
                </div>
              )}
            </>
          )}
          {chosen.agentId && (
            <div className="admin-node-controls">
              {chosen.taskId && (
                <button
                  className="icon-button"
                  onClick={() => onTaskCancel(chosen.taskId!)}
                >
                  Stop agent process
                </button>
              )}
              <button
                className="icon-button"
                onClick={() => {
                  if (
                    window.confirm(
                      `Revoke ${chosen.name} and its guest access?`,
                    )
                  )
                    onAgentRevoke(chosen.agentId!);
                }}
              >
                Revoke agent
              </button>
            </div>
          )}
          {chosen.group && (
            <p>
              {chosen.group.members.length} members ·{" "}
              {chosen.group.files.length} explicitly shared, read-only files.
              Manage membership and grants in Permissions below.
            </p>
          )}
          <h4>Recent work</h4>
          {current.length ? (
            <ul>
              {current.slice(-5).map((t) => (
                <li key={t.id}>
                  <strong>{t.title}</strong>
                  <span>
                    {t.status} · {t.kind.replaceAll("_", " ")}
                  </span>
                </li>
              ))}
            </ul>
          ) : (
            <p>No recorded tasks for this element.</p>
          )}
        </aside>
      </div>
      <p className="admin-canvas-caption">
        Select a node to inspect and control it. Moving paths show running tasks
        reported by the hub; they do not claim packet-level transfer progress.
      </p>
    </section>
  );
}
