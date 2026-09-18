/* Developed with care by FACRF - https://github.com/facrf */
import cytoscape from "cytoscape";
import type { Core, CollectionReturnValue, ElementDefinition, StylesheetJson } from "cytoscape";
import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { apiUrl } from "../services/api";
import type { GrafoNode, GrafoResponse } from "../types/api";
import { GraphVitalityBar, percentual } from "./PsychosocialStatus";

export type GraphLayout = "force" | "hierarchical";
interface GraphViewerProps {
  graph: GrafoResponse;
  layout: GraphLayout;
  focusedNodeId: number | null;
  groupByCategory: boolean;
  expanded: boolean;
  onEdgeClick: (edgeId: number) => void;
  onNodeDoubleClick: (nodeId: number) => void;
  onNodeSelect: (nodeId: number) => void;
}
export interface GraphViewerHandle { exportPng: () => string | null; organize: () => void; fit: () => void }
interface Overlay { id: number; x: number; y: number; zoom: number; hp: number; dimmed: boolean }
interface GroupLabel { name: string; x: number; y: number; zoom: number }

export const GraphViewer = forwardRef<GraphViewerHandle, GraphViewerProps>(function GraphViewer(props, ref) {
  const { graph, layout, focusedNodeId, groupByCategory, expanded } = props;
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const callbacksRef = useRef(props);
  const structureRef = useRef("");
  const syncOverlaysRef = useRef<(() => void) | null>(null);
  const [overlays, setOverlays] = useState<Overlay[]>([]);
  const [groupLabels, setGroupLabels] = useState<GroupLabel[]>([]);

  useEffect(() => { callbacksRef.current = props; }, [props]);
  useImperativeHandle(ref, () => ({ organize: () => {
    const cy = cyRef.current;
    if (cy) organizeGraph(cy, callbacksRef.current.graph, callbacksRef.current.layout, callbacksRef.current.groupByCategory);
  }, fit: () => {
    const cy = cyRef.current;
    if (cy) { cy.resize(); fitGraph(cy); }
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
    const observer = new ResizeObserver(() => { cy.resize(); fitGraph(cy); syncOverlays(); });
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
    if (structure !== structureRef.current) {
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
      const neighborhood = focus.union(direct).union(secondary);
      cy.nodes().difference(neighborhood).addClass("is-dimmed");
      cy.edges().difference(neighborhood.edgesWith(neighborhood)).addClass("is-dimmed");
      const data = focus.data("profile") as GrafoNode;
      for (const row of data.contribuicoes) cy.$id(`node-${row.fonte_id}`).addClass("is-source");
    });
    // Synchronize DOM bars after every snapshot, even when geometry is unchanged.
    syncOverlaysRef.current?.();
  }, [graph, layout, focusedNodeId, groupByCategory]);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    cy.nodes().unselect();
    if (focusedNodeId === null) return;
    const node = cy.$id(`node-${focusedNodeId}`);
    if (node.empty()) return;
    node.select();
    // Selecting a node highlights it without moving an already adjusted viewport.
  }, [focusedNodeId]);

  return <div className={`relative h-full w-full overflow-hidden ${expanded ? "min-h-80" : "min-h-[38rem]"}`}>
    <div ref={containerRef} className={`h-full w-full cursor-grab active:cursor-grabbing ${expanded ? "min-h-80" : "min-h-[38rem]"}`} role="application" aria-label="Grafo interativo de relacionamentos" />
    <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
      {groupLabels.map(({ name, x, y, zoom }) => <div key={name} data-graph-category={name} title={name} className="absolute max-w-[200px] truncate rounded-full border border-teal-200 bg-white/95 px-3 py-1 text-xs font-semibold text-teal-900 shadow-sm" style={{ left: x, top: y, transform: `scale(${zoom}) translate(-50%, -100%)`, transformOrigin: "top left" }}>{name}</div>)}
      {overlays.map(({ id, x, y, zoom, hp, dimmed }) => <div key={id} data-person-id={id} className="absolute w-14" style={{ left: x, top: y, opacity: dimmed ? 0.2 : 1, transform: `scale(${zoom}) translateX(-50%)`, transformOrigin: "top left" }}>
        <GraphVitalityBar hp={hp} />
        <span className="mt-1 block text-center text-[10px] tabular-nums text-slate-600">{percentual(hp)}</span>
      </div>)}
    </div>
  </div>;
});

function fitGraph(cy: Core) {
  if (cy.elements().empty() || cy.width() <= 0 || cy.height() <= 0) return;
  // Include DOM vitality bars beneath the nodes in the viewport calculation.
  const bounds = cy.elements().boundingBox();
  if (cy.scratch("groupByCategory")) {
    for (const [, nodes] of categoryGroups(cy)) {
      const group = nodes.boundingBox();
      const center = (group.x1 + group.x2) / 2;
      bounds.x1 = Math.min(bounds.x1, center - 100);
      bounds.x2 = Math.max(bounds.x2, center + 100);
    }
    bounds.w = bounds.x2 - bounds.x1;
  }
  const padding = Math.min(60, cy.width() / 4, cy.height() / 4);
  const zoom = Math.min(cy.maxZoom(), (cy.width() - 2 * padding) / (bounds.w + 20),
    (cy.height() - 2 * padding) / (bounds.h + 160));
  cy.viewport({ zoom, pan: {
    x: (cy.width() - zoom * (bounds.x1 + bounds.x2)) / 2,
    y: (cy.height() - zoom * (bounds.y1 + bounds.y2)) / 2,
  } });
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
    const columns = Math.ceil(Math.sqrt(groups.length));
    const width = Math.max(...groups.map(group => group.bounds.w), 200) + 160;
    const height = Math.max(...groups.map(group => group.bounds.h), 150) + 180;
    cy.batch(() => groups.forEach(({ nodes, bounds }, index) => {
      nodes.positions(node => ({ x: node.position("x") - bounds.x1 + index % columns * width,
        y: node.position("y") - bounds.y1 + Math.floor(index / columns) * height }));
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
