/**
 * GraphView — S39: Note graph with backlinks + semantic similarity edges.
 *
 * Pure canvas rendering with a simple spring-force simulation.
 * No external graph library required.
 */
import { type Component, createEffect, createResource, onCleanup, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { Note } from "@/types/note";

// ─── Types ────────────────────────────────────────────────────────────────────

interface GraphNode {
  id: string;
  title: string;
  provider_id: string;
  color: string;
}

interface GraphEdge {
  source: string;
  target: string;
  kind: "backlink" | "semantic";
  weight: number;
}

interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

interface Props {
  onNoteClick?: (noteId: string) => void;
}

// ─── Color map ────────────────────────────────────────────────────────────────

const COLOR_MAP: Record<string, string> = {
  default: "#4a4a6a",
  red: "#7a3030",
  orange: "#6a4a20",
  yellow: "#6a5a20",
  green: "#2a6040",
  teal: "#2a5a60",
  blue: "#2a4070",
  dark_blue: "#1a2850",
  purple: "#4a2a70",
  pink: "#7a2a5a",
  brown: "#5a4a2a",
  gray: "#404040",
};

const PROVIDER_COLORS: Record<string, string> = {
  local: "#4a8a5a",
  keep: "#8a6a20",
  onenote: "#2060a0",
  notion: "#a0a0a0",
  obsidian: "#7a50c0",
  markdown: "#508050",
  windows_sticky_notes: "#d0b030",
};

// ─── Force simulation ─────────────────────────────────────────────────────────

interface SimNode extends GraphNode {
  x: number;
  y: number;
  vx: number;
  vy: number;
  fx?: number;
  fy?: number;
}

function initSim(nodes: GraphNode[], edges: GraphEdge[], width: number, height: number) {
  const sim: SimNode[] = nodes.map((n) => ({
    ...n,
    x: width / 2 + (Math.random() - 0.5) * 400,
    y: height / 2 + (Math.random() - 0.5) * 400,
    vx: 0,
    vy: 0,
  }));

  const idxById: Record<string, number> = {};
  sim.forEach((n, i) => { idxById[n.id] = i; });

  const edgeLinks = edges
    .map((e) => ({ s: idxById[e.source], t: idxById[e.target], kind: e.kind, weight: e.weight }))
    .filter((e) => e.s !== undefined && e.t !== undefined);

  function tick() {
    const cx = width / 2;
    const cy = height / 2;
    const alpha = 0.3;

    // Gravity toward center
    for (const n of sim) {
      if (n.fx !== undefined) { n.x = n.fx; n.vy = 0; continue; }
      n.vx += (cx - n.x) * 0.002;
      n.vy += (cy - n.y) * 0.002;
    }

    // Repulsion between nodes
    for (let i = 0; i < sim.length; i++) {
      for (let j = i + 1; j < sim.length; j++) {
        const dx = sim[j].x - sim[i].x;
        const dy = sim[j].y - sim[i].y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = -3000 / (dist * dist);
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;
        sim[i].vx += fx;
        sim[i].vy += fy;
        sim[j].vx -= fx;
        sim[j].vy -= fy;
      }
    }

    // Spring forces for edges
    for (const { s, t, weight } of edgeLinks) {
      const a = sim[s];
      const b = sim[t];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const restLen = 120 + (1 - weight) * 60;
      const force = (dist - restLen) * 0.08;
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;
      a.vx += fx;
      a.vy += fy;
      b.vx -= fx;
      b.vy -= fy;
    }

    // Integrate + damping
    for (const n of sim) {
      n.vx *= 0.85;
      n.vy *= 0.85;
      n.x += n.vx;
      n.y += n.vy;
    }

    return sim;
  }

  return { sim, tick, idxById, edgeLinks };
}

// ─── Component ────────────────────────────────────────────────────────────────

export const GraphView: Component<Props> = (props) => {
  let canvas!: HTMLCanvasElement;
  let raf: number;
  let simState: ReturnType<typeof initSim> | null = null;

  // View transform
  let tx = 0, ty = 0, scale = 1;
  let isPanning = false;
  let panStart = { x: 0, y: 0 };
  let draggingIdx: number | null = null;

  const [data] = createResource<GraphData>(() =>
    invoke<GraphData>("notes_get_graph")
  );

  function toWorld(px: number, py: number) {
    return { x: (px - tx) / scale, y: (py - ty) / scale };
  }

  function draw(ctx: CanvasRenderingContext2D, w: number, h: number) {
    ctx.clearRect(0, 0, w, h);
    ctx.save();
    ctx.translate(tx, ty);
    ctx.scale(scale, scale);

    const state = simState;
    if (!state) { ctx.restore(); return; }

    // Draw edges
    for (const { s, t, kind, weight } of state.edgeLinks) {
      const a = state.sim[s];
      const b = state.sim[t];
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);

      if (kind === "backlink") {
        ctx.strokeStyle = "rgba(120,180,255,0.5)";
        ctx.lineWidth = 1.5;
        ctx.setLineDash([]);
      } else {
        ctx.strokeStyle = `rgba(180,120,255,${0.2 + weight * 0.4})`;
        ctx.lineWidth = 1;
        ctx.setLineDash([4, 4]);
      }
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Draw nodes
    const NODE_R = 18;
    for (const n of state.sim) {
      const fill = PROVIDER_COLORS[n.provider_id] ?? COLOR_MAP[n.color] ?? "#4a4a6a";
      ctx.beginPath();
      ctx.arc(n.x, n.y, NODE_R, 0, Math.PI * 2);
      ctx.fillStyle = fill;
      ctx.fill();
      ctx.strokeStyle = "rgba(255,255,255,0.2)";
      ctx.lineWidth = 1;
      ctx.stroke();

      // Label
      ctx.fillStyle = "rgba(255,255,255,0.9)";
      ctx.font = "11px system-ui, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      const label = n.title.length > 18 ? n.title.slice(0, 15) + "…" : n.title;
      ctx.fillText(label, n.x, n.y + NODE_R + 3);
    }

    ctx.restore();
  }

  function loop(ctx: CanvasRenderingContext2D, w: number, h: number) {
    if (simState) simState.tick();
    draw(ctx, w, h);
    raf = requestAnimationFrame(() => loop(ctx, w, h));
  }

  function nodeAt(worldX: number, worldY: number): number | null {
    const state = simState;
    if (!state) return null;
    for (let i = 0; i < state.sim.length; i++) {
      const n = state.sim[i];
      const dx = worldX - n.x;
      const dy = worldY - n.y;
      if (Math.sqrt(dx * dx + dy * dy) < 20) return i;
    }
    return null;
  }

  onMount(() => {
    const ctx = canvas.getContext("2d")!;
    const resize = () => {
      canvas.width = canvas.offsetWidth;
      canvas.height = canvas.offsetHeight;
    };
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);
    resize();

    // Start loop
    raf = requestAnimationFrame(() => loop(ctx, canvas.width, canvas.height));

    // Mouse events
    canvas.addEventListener("mousedown", (e) => {
      const world = toWorld(e.offsetX, e.offsetY);
      const hit = nodeAt(world.x, world.y);
      if (hit !== null) {
        draggingIdx = hit;
        if (simState) {
          simState.sim[hit].fx = world.x;
          simState.sim[hit].fy = world.y;
        }
      } else {
        isPanning = true;
        panStart = { x: e.clientX - tx, y: e.clientY - ty };
      }
    });

    canvas.addEventListener("mousemove", (e) => {
      if (draggingIdx !== null && simState) {
        const world = toWorld(e.offsetX, e.offsetY);
        simState.sim[draggingIdx].fx = world.x;
        simState.sim[draggingIdx].fy = world.y;
        simState.sim[draggingIdx].x = world.x;
        simState.sim[draggingIdx].y = world.y;
      } else if (isPanning) {
        tx = e.clientX - panStart.x;
        ty = e.clientY - panStart.y;
      }
    });

    canvas.addEventListener("mouseup", (e) => {
      if (draggingIdx !== null) {
        const world = toWorld(e.offsetX, e.offsetY);
        if (simState) {
          simState.sim[draggingIdx].fx = undefined;
          simState.sim[draggingIdx].fy = undefined;
        }
        // Check if it was a click (didn't move much)
        const n = simState?.sim[draggingIdx];
        if (n && Math.abs(n.x - world.x) < 5 && Math.abs(n.y - world.y) < 5) {
          props.onNoteClick?.(n.id);
        }
        draggingIdx = null;
      }
      isPanning = false;
    });

    canvas.addEventListener("wheel", (e) => {
      e.preventDefault();
      const delta = e.deltaY > 0 ? 0.9 : 1.1;
      const mx = e.offsetX;
      const my = e.offsetY;
      tx = mx - (mx - tx) * delta;
      ty = my - (my - ty) * delta;
      scale *= delta;
    }, { passive: false });

    onCleanup(() => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    });
  });

  createEffect(() => {
    const d = data();
    if (!d || !canvas) return;
    simState = initSim(d.nodes, d.edges, canvas.offsetWidth || 800, canvas.offsetHeight || 600);
    // Center the initial view
    tx = 0; ty = 0; scale = 1;
  });

  return (
    <div class="graph-view">
      <div class="graph-view__legend">
        <span class="graph-view__legend-item graph-view__legend-item--backlink">── backlink</span>
        <span class="graph-view__legend-item graph-view__legend-item--semantic">- - semántico</span>
        <span class="graph-view__legend-hint">Arrastrá nodos · Scroll para zoom · Click para abrir</span>
      </div>
      <canvas ref={canvas} class="graph-view__canvas" />
    </div>
  );
};
