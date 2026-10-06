/* Developed with care by FACRF - https://github.com/facrf */
import cytoscape from "cytoscape";
import type { Core, CollectionReturnValue, ElementDefinition, StylesheetJson } from "cytoscape";
import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { Minus, Plus, Scan } from "lucide-react";
import { apiUrl } from "../services/api";
import type { GrafoNode, GrafoResponse, PosicaoGrafo } from "../types/api";
import { GraphVitalityBar, percentual } from "./PsychosocialStatus";

export type GraphLayout = "force" | "hierarchical";
interface GraphViewerProps {
  graph: GrafoResponse;
  layout: GraphLayout;
  focusedNodeId: number | null;
  focusDepth: number;
  focusRevision: number;
  groupByCategory: boolean;
  expanded: boolean;
  onEdgeClick: (edgeId: number) => void;
  onNodeDoubleClick: (nodeId: number) => void;
  onNodeSelect: (nodeId: number) => void;
}
export interface GraphViewerHandle { exportPng: () => string | null; organize: () => void; fit: () => void; positions: () => PosicaoGrafo[]; restorePositions: (positions: PosicaoGrafo[]) => boolean }
interface Overlay { id: number; x: number; y: number; zoom: number; hp: number; dimmed: boolean }
interface GroupLabel { name: string; x: number; y: number; zoom: number }

export const GraphViewer = forwardRef<GraphViewerHandle, GraphViewerProps>(function GraphViewer(props, ref) {
  const { graph, layout, focusedNodeId, focusDepth, focusRevision, groupByCategory, expanded } = props;
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const callbacksRef = useRef(props);
  const structureRef = useRef("");
  const syncOverlaysRef = useRef<(() => void) | null>(null);
  const [overlays, setOverlays] = useState<Overlay[]>([]);
  const [groupLabels, setGroupLabels] = useState<GroupLabel[]>([]);
  const [currentZoom, setCurrentZoom] = useState(1);

  useEffect(() => { callbacksRef.current = props; }, [props]);
  useImperativeHandle(ref, () => ({ organize: () => {
    const cy = cyRef.current;
    if (cy) {
      const current = callbacksRef.current;
      organizeGraph(cy, current.graph, current.layout, current.groupByCategory);
      if (current.focusedNodeId !== null) focusGraph(cy, current.focusedNodeId, current.focusDepth);
    }
  }, fit: () => {
    const cy = cyRef.current;
    if (cy) { cy.resize(); fitGraph(cy); }
  }, positions: () => cyRef.current?.nodes().map(node => ({ pessoa_id: Number(node.data("nodeId")), ...node.position() })) ?? [],
  restorePositions: (positions) => {
    const cy = cyRef.current;
    if (!cy) return false;
    const saved = new Map(positions.filter(p => Number.isFinite(p.x) && Number.isFinite(p.y)).map(p => [`node-${p.pessoa_id}`, p]));
    if (cy.nodes().filter(node => saved.has(node.id())).empty()) return false;
    cy.batch(() => cy.nodes().positions(node => {
      const position = saved.get(node.id());
      return position ? { x: position.x, y: position.y } : node.position();
    }));
    const current = callbacksRef.current;
    if (current.focusedNodeId !== null) focusGraph(cy, current.focusedNodeId, current.focusDepth);
    else fitGraph(cy);
    return true;
  }, exportPng: () => {
    const cy = cyRef.current;
    if (!cy) return null;
    const result = cy.png({ bg: "#FFFFFF", full: true, maxHeight: 1800, maxWidth: 2400, scale: 2 });
    return typeof result === "string" ? result : null;
  }}), []);

  // Keep one Cytoscape instance so data refreshes retain the viewport and selection.
  useEffect(() => {
    if (!containerRef.current) return;
    const cy = cytoscape({ container: containerRef.current, elements: [], style: graphStyles,
      maxZoom: 3, wheelSensitivity: 0.18, boxSelectionEnabled: false });
    cyRef.current = cy;
    let frame = 0;
    const syncOverlays = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const zoom = cy.zoom();
        setCurrentZoom(zoom);
        const next = cy.nodes().map((item) => {
          const node = item.data("profile") as GrafoNode;
          const pos = item.renderedPosition();
          return { id: node.id, x: pos.x, y: pos.y + item.renderedOuterHeight() / 2 + 32 * zoom, zoom, hp: node.hp, dimmed: item.hasClass("is-dimmed") };
        });
        setOverlays((previous) => JSON.stringify(previous) === JSON.stringify(next) ? previous : next);
        const labels = callbacksRef.current.groupByCategory ? categoryGroups(cy).map(([name, nodes]) => {
          const bounds = nodes.boundingBox();
          const pan = cy.pan();
          return { name, x: (bounds.x1 + bounds.x2) / 2 * zoom + pan.x,
            y: (bounds.y1 - 35) * zoom + pan.y, zoom };
        }) : [];
        setGroupLabels(previous => JSON.stringify(previous) === JSON.stringify(labels) ? previous : labels);
      });
    };
    syncOverlaysRef.current = syncOverlays;
    cy.on("render pan zoom position data", syncOverlays);
    cy.on("dragpan scrollzoom pinchzoom", () => cy.scratch("viewportMode", "manual"));
    cy.on("mouseover", "node, edge", (event) => event.target.addClass("is-hover"));
    cy.on("mouseout", "node, edge", (event) => event.target.removeClass("is-hover"));
    let lastTap = { id: "", at: 0 };
    cy.on("tap", "edge", (event) => callbacksRef.current.onEdgeClick(Number(event.target.data("edgeId"))));
    cy.on("tap", "node", (event) => {
      const id = String(event.target.id());
      const now = Date.now();
      if (lastTap.id === id && now - lastTap.at < 350) {
        callbacksRef.current.onNodeDoubleClick(Number(event.target.data("nodeId")));
        lastTap = { id: "", at: 0 };
      } else {
        lastTap = { id, at: now };
        callbacksRef.current.onNodeSelect(Number(event.target.data("nodeId")));
      }
    });
    const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    let timer: number | undefined;
    const applyMotion = () => {
      window.clearInterval(timer);
      cy.nodes().style("underlay-opacity", 0.16);
      if (!motion.matches) timer = window.setInterval(() => {
        if (document.hidden) return;
        cy.nodes("[?auraPulsante]").style("underlay-opacity", 0.18 + Math.sin(performance.now() / 573) * 0.06);
      }, 120);
    };
    motion.addEventListener("change", applyMotion);
    applyMotion();
    let viewportWidth = cy.width();
    let viewportHeight = cy.height();
    const observer = new ResizeObserver(() => {
      const pan = cy.pan();
      cy.resize();
      if (cy.scratch("viewportMode") === "manual") {
        cy.pan({ x: pan.x + (cy.width() - viewportWidth) / 2,
          y: pan.y + (cy.height() - viewportHeight) / 2 });
      } else if (cy.scratch("viewportMode") === "focus") {
        const { focusedNodeId, focusDepth } = callbacksRef.current;
        if (focusedNodeId !== null) focusGraph(cy, focusedNodeId, focusDepth);
        else fitGraph(cy);
      } else fitGraph(cy);
      viewportWidth = cy.width();
      viewportHeight = cy.height();
      syncOverlays();
    });
    observer.observe(containerRef.current);
    return () => {
      window.clearInterval(timer); cancelAnimationFrame(frame);
      motion.removeEventListener("change", applyMotion); observer.disconnect();
      syncOverlaysRef.current = null;
      cy.destroy(); cyRef.current = null; structureRef.current = "";
    };
  }, []);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    const elements: ElementDefinition[] = [
      ...graph.nodes.map((node) => ({ data: { id: `node-${node.id}`, nodeId: node.id,
        label: node.label,
        color: node.color || "#86A6A3", image: node.foto_url ? apiUrl(node.foto_url) : "none",
        legalEntity: node.pessoa_juridica, auraColor: node.aura_cor_hex, auraPulsante: node.aura_pulsante, profile: node } })),
      ...graph.edges.map((edge) => ({ data: { id: `edge-${edge.id}`, edgeId: edge.id,
        source: `node-${edge.source}`, target: `node-${edge.target}`, label: edge.label } })),
    ];
    const ids = new Set(elements.map((element) => element.data.id));
    cy.batch(() => {
      cy.elements().filter((item) => !ids.has(item.id())).remove();
      for (const element of elements) {
        const existing = cy.$id(element.data.id!);
        if (existing.empty()) cy.add(element);
        else if (existing.isEdge() && (existing.data("source") !== element.data.source || existing.data("target") !== element.data.target)) { existing.remove(); cy.add(element); }
        else existing.data(element.data);
      }
    });
    cy.nodes("[!auraPulsante]").style("underlay-opacity", 0.16);
    const structure = `${layout}:${groupByCategory}:${graph.nodes.map((node) => `${node.id}${groupByCategory ? `-${node.categoria}` : ""}`).sort().join(",")}:${graph.edges.map((edge) => `${edge.id}-${edge.source}-${edge.target}`).sort().join(",")}`;
    const structureChanged = structure !== structureRef.current;
    if (structureChanged) {
      structureRef.current = structure;
      organizeGraph(cy, graph, layout, groupByCategory);
    }
    // Highlight source profiles and the selected node's first/second degree neighborhood.
    cy.batch(() => {
      cy.elements().removeClass("is-source is-direct is-secondary focus-edge second-edge is-dimmed");
      if (focusedNodeId === null) return;
      const focus = cy.$id(`node-${focusedNodeId}`);
      if (focus.empty()) return;
      const direct = focus.neighborhood("node");
      const secondary = direct.neighborhood("node").difference(direct).difference(focus);
      direct.addClass("is-direct"); secondary.addClass("is-secondary");
      focus.connectedEdges().addClass("focus-edge");
      direct.connectedEdges().difference(focus.connectedEdges()).addClass("second-edge");
      const neighborhood = connectionNodes(focus, focusDepth);
      cy.nodes().difference(neighborhood).addClass("is-dimmed");
      cy.edges().difference(neighborhood.edgesWith(neighborhood)).addClass("is-dimmed");
      const data = focus.data("profile") as GrafoNode;
      for (const row of data.contribuicoes) cy.$id(`node-${row.fonte_id}`).addClass("is-source");
    });
    cy.nodes().unselect();
    if (focusedNodeId !== null) cy.$id(`node-${focusedNodeId}`).select();
    if (structureChanged && focusedNodeId !== null) focusGraph(cy, focusedNodeId, focusDepth);
    // Synchronize DOM bars after every snapshot, even when geometry is unchanged.
    syncOverlaysRef.current?.();
  }, [graph, layout, focusedNodeId, focusDepth, groupByCategory]);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    cy.nodes().unselect();
    if (focusedNodeId === null) { fitGraph(cy); return; }
    const node = cy.$id(`node-${focusedNodeId}`);
    if (node.empty()) return;
    node.select();
    focusGraph(cy, focusedNodeId, focusDepth);
  }, [focusedNodeId, focusDepth, focusRevision]);

  const changeZoom = (factor: number) => {
    const cy = cyRef.current;
    if (!cy) return;
    cy.scratch("viewportMode", "manual");
    cy.zoom({ level: Math.max(cy.minZoom(), Math.min(cy.maxZoom(), cy.zoom() * factor)),
      renderedPosition: { x: cy.width() / 2, y: cy.height() / 2 } });
  };

  return <div className={`relative h-full w-full overflow-hidden ${expanded ? "min-h-80" : ""}`}>
    <div ref={containerRef} className="h-full w-full cursor-grab active:cursor-grabbing" role="application" aria-label="Grafo interativo de relacionamentos" />
    <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
      {groupLabels.map(({ name, x, y, zoom }) => <div key={name} data-graph-category={name} title={name} className="absolute max-w-[200px] truncate rounded-full border border-teal-200 bg-white/95 px-3 py-1 text-xs font-semibold text-teal-900 shadow-sm" style={{ left: x, top: y, transform: `scale(${zoom}) translate(-50%, -100%)`, transformOrigin: "top left" }}>{name}</div>)}
      {overlays.map(({ id, x, y, zoom, hp, dimmed }) => <div key={id} data-person-id={id} className="absolute w-14" style={{ left: x, top: y, opacity: dimmed ? 0.2 : 1, transform: `scale(${zoom}) translateX(-50%)`, transformOrigin: "top left" }}>
        <GraphVitalityBar hp={hp} />
        <span className="mt-1 block text-center text-[10px] tabular-nums text-slate-600">{percentual(hp)}</span>
      </div>)}
    </div>
    <div className="absolute right-3 top-3 flex items-center gap-1 rounded-xl border border-slate-200 bg-white/95 p-1 shadow-sm" role="group" aria-label="Controles de zoom">
      <button type="button" className="icon-button" aria-label="Diminuir zoom" onClick={() => changeZoom(1 / 1.25)}><Minus className="size-4" /></button>
      <output className="min-w-12 text-center text-xs tabular-nums text-slate-600" aria-label="Zoom atual">{Math.round(currentZoom * 100)}%</output>
      <button type="button" className="icon-button" aria-label="Aumentar zoom" onClick={() => changeZoom(1.25)}><Plus className="size-4" /></button>
      <button type="button" className="icon-button" aria-label="Enquadrar mapa" onClick={() => { if (cyRef.current) fitGraph(cyRef.current); }}><Scan className="size-4" /></button>
    </div>
  </div>;
});

function fitGraph(cy: Core) {
  if (cy.elements().empty() || cy.width() <= 0 || cy.height() <= 0) return;
  const bounds = visualBounds(cy, cy.elements("node"));
  const padding = Math.min(24, cy.width() / 8, cy.height() / 8);
  const zoom = Math.min(2, cy.maxZoom(), (cy.width() - 2 * padding) / bounds.w,
    (cy.height() - 2 * padding) / bounds.h);
  cy.scratch("viewportMode", "all");
  cy.viewport({ zoom, pan: {
    x: (cy.width() - zoom * (bounds.x1 + bounds.x2)) / 2,
    y: (cy.height() - zoom * (bounds.y1 + bounds.y2)) / 2,
  } });
}

// Keep the person at the viewport center while fitting the visible connections.
function focusGraph(cy: Core, nodeId: number, depth: number) {
  const focus = cy.$id(`node-${nodeId}`);
  if (focus.empty() || cy.width() <= 0 || cy.height() <= 0) return;
  const nodes = connectionNodes(focus, depth);
  const bounds = visualBounds(cy, nodes);
  const center = focus.position();
  // Reserve space for labels, auras and DOM vitality bars below each node.
  const halfWidth = Math.max(center.x - bounds.x1, bounds.x2 - center.x);
  const halfHeight = Math.max(center.y - bounds.y1, bounds.y2 - center.y);
  const padding = Math.min(24, cy.width() / 8, cy.height() / 8);
  const zoom = Math.min(2, cy.maxZoom(), (cy.width() - 2 * padding) / (2 * halfWidth),
    (cy.height() - 2 * padding) / (2 * halfHeight));
  cy.scratch("viewportMode", "focus");
  cy.viewport({ zoom, pan: {
    x: cy.width() / 2 - center.x * zoom,
    y: cy.height() / 2 - center.y * zoom,
  } });
}

function connectionNodes(focus: CollectionReturnValue, depth: number) {
  let nodes = focus;
  for (let level = 0; level < depth; level += 1) nodes = nodes.union(nodes.neighborhood("node"));
  return nodes;
}

// Account for the actual DOM decorations instead of adding a fixed empty band.
function visualBounds(cy: Core, nodes: CollectionReturnValue) {
  // A single element can return Cytoscape's cached box; keep our visual margins
  // separate so repeated framing cannot enlarge the renderer's own bounds.
  const bounds = { ...nodes.union(nodes.edgesWith(nodes)).boundingBox() };
  nodes.forEach(node => {
    const position = node.position();
    bounds.x1 = Math.min(bounds.x1, position.x - 28);
    bounds.x2 = Math.max(bounds.x2, position.x + 28);
    bounds.y2 = Math.max(bounds.y2, position.y + node.outerHeight() / 2 + 58);
  });
  if (cy.scratch("groupByCategory")) {
    for (const [, groupNodes] of categoryGroups(cy)) {
      if (groupNodes.intersection(nodes).empty()) continue;
      const group = groupNodes.boundingBox();
      const center = (group.x1 + group.x2) / 2;
      bounds.x1 = Math.min(bounds.x1, center - 100);
      bounds.x2 = Math.max(bounds.x2, center + 100);
      bounds.y1 = Math.min(bounds.y1, group.y1 - 67);
    }
  }
  bounds.x1 -= 10; bounds.x2 += 10;
  bounds.y1 -= 10; bounds.y2 += 10;
  bounds.w = bounds.x2 - bounds.x1;
  bounds.h = bounds.y2 - bounds.y1;
  return bounds;
}

function categoryGroups(cy: Core): [string, CollectionReturnValue][] {
  const groups = new Map<string, CollectionReturnValue>();
  cy.nodes().forEach(node => {
    const name = (node.data("profile") as GrafoNode).categoria || "Sem categoria";
    groups.set(name, (groups.get(name) ?? cy.collection()).union(node));
  });
  return [...groups].sort(([a], [b]) => a.localeCompare(b, "pt-BR"));
}

function organizeGraph(cy: Core, graph: GrafoResponse, layout: GraphLayout, grouped: boolean) {
  if (cy.nodes().empty()) return;
  cy.scratch("groupByCategory", grouped);
  cy.resize();
  if (grouped) {
    const groups = categoryGroups(cy).map(([, nodes]) => {
      const ids = new Set(nodes.map(node => Number(node.data("nodeId"))));
      const subset = { ...graph, nodes: graph.nodes.filter(node => ids.has(node.id)),
        edges: graph.edges.filter(edge => ids.has(edge.source) && ids.has(edge.target)) };
      runLayout(nodes.union(nodes.edgesWith(nodes)), subset, layout);
      return { nodes, bounds: nodes.boundingBox() };
    });
    const averageWidth = groups.reduce((sum, group) => sum + Math.max(group.bounds.w, 200) + 80, 0) / groups.length;
    const averageHeight = groups.reduce((sum, group) => sum + group.bounds.h + 140, 0) / groups.length;
    const aspect = cy.width() / Math.max(1, cy.height());
    const columns = Math.max(1, Math.min(groups.length, Math.round(Math.sqrt(groups.length * aspect * averageHeight / averageWidth))));
    let x = 0, y = 0, rowHeight = 0;
    cy.batch(() => groups.forEach(({ nodes, bounds }, index) => {
      if (index > 0 && index % columns === 0) { x = 0; y += rowHeight; rowHeight = 0; }
      const width = Math.max(bounds.w, 200);
      nodes.positions(node => ({ x: node.position("x") - bounds.x1 + x + (width - bounds.w) / 2,
        y: node.position("y") - bounds.y1 + y }));
      x += width + 80;
      rowHeight = Math.max(rowHeight, bounds.h + 140);
    }));
  } else runLayout(cy.elements(), graph, layout);
  fitGraph(cy);
}

function runLayout(elements: CollectionReturnValue, graph: GrafoResponse, layout: GraphLayout) {
  const roots = hierarchicalRoots(graph);
  const options = { animate: false, fit: false, nodeDimensionsIncludeLabels: true };
  elements.layout(layout === "force" ? { ...options, name: "cose", randomize: true,
    idealEdgeLength: 155, nodeRepulsion: 8000, gravity: 0.22, numIter: 1200, componentSpacing: 100 }
    : { ...options, name: "breadthfirst", directed: true, spacingFactor: 1.65,
      avoidOverlap: true, roots: roots.length ? roots : undefined }).run();
}

function hierarchicalRoots(graph: GrafoResponse): string[] {
  const targets = new Set(graph.edges.map((edge) => edge.target));
  const roots = graph.nodes.filter((node) => !targets.has(node.id));
  const selectedRoots = roots.length > 0 ? roots : graph.nodes.slice(0, 1);
  return selectedRoots.map((node) => `node-${node.id}`);
}

const graphStyles: StylesheetJson = [
  {
    selector: "node",
    style: {
      width: 68,
      height: 68,
      "background-color": "#E7EFED",
      "background-image": "data(image)",
      "background-fit": "cover",
      "background-clip": "node",
      "border-width": 5,
      "border-color": "data(color)",
      "underlay-color": "data(auraColor)",
      "underlay-opacity": 0.16,
      "underlay-padding": 10,
      "underlay-shape": "ellipse",
      label: "data(label)",
      color: "#193837",
      "font-size": 12,
      "font-weight": 650,
      "text-valign": "bottom",
      "text-margin-y": 11,
      "text-background-color": "#FFFFFF",
      "text-background-opacity": 0.92,
      "text-background-padding": "5px",
      "text-background-shape": "roundrectangle",
      "overlay-opacity": 0,
      "transition-property": "width height border-width border-color",
      "transition-duration": 160,
    },
  },
  {
    selector: "node.is-secondary",
    style: { "border-style": "dashed" },
  },
  {
    selector: "node.is-source",
    style: { "border-width": 8, "font-weight": 750 },
  },
  {
    selector: "node[?legalEntity]",
    style: {
      shape: "round-rectangle",
      "underlay-shape": "round-rectangle",
    },
  },
  {
    selector: "node.is-hover",
    style: { width: 76, height: 76, "border-width": 7 },
  },
  {
    selector: "node:selected",
    style: {
      width: 78,
      height: 78,
      "border-color": "data(color)",
      "border-width": 7,
      "overlay-color": "#193837",
      "overlay-opacity": 0.12,
      "overlay-padding": 8,
    },
  },
  {
    selector: "edge",
    style: {
      width: 2.5,
      "line-color": "#9BB5B2",
      "target-arrow-color": "#9BB5B2",
      "target-arrow-shape": "triangle",
      "arrow-scale": 0.85,
      "curve-style": "bezier",
      label: "data(label)",
      color: "#526866",
      "font-size": 10,
      "font-weight": 600,
      "text-rotation": "autorotate",
      "text-background-color": "#F8F7F2",
      "text-background-opacity": 0.94,
      "text-background-padding": "4px",
      "text-margin-y": -9,
      "overlay-opacity": 0,
      "transition-property": "width line-color target-arrow-color",
      "transition-duration": 140,
    },
  },
  {
    selector: "edge.focus-edge",
    style: { "line-color": "#0F766E", "target-arrow-color": "#0F766E", width: 3 },
  },
  {
    selector: "edge.second-edge",
    style: { "line-style": "dashed", "line-color": "#94A3B8", "target-arrow-color": "#94A3B8" },
  },
  {
    selector: ".is-dimmed",
    style: { opacity: 0.2 },
  },
  {
    selector: "edge.is-hover, edge:selected",
    style: {
      width: 4,
      "line-color": "#E7654F",
      "target-arrow-color": "#E7654F",
      color: "#B83E2D",
    },
  },
];
