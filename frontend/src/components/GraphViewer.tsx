/* Developed with care by FACRF - https://github.com/facrf */
import cytoscape from "cytoscape";
import type { Core, ElementDefinition, StylesheetJson } from "cytoscape";
import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { apiUrl } from "../services/api";
import type { GrafoNode, GrafoResponse, PosicaoGrafo } from "../types/api";
import { VitalityBar } from "./PsychosocialStatus";

export type GraphLayout = "force" | "hierarchical";
interface GraphViewerProps {
  graph: GrafoResponse;
  layout: GraphLayout;
  focusedNodeId: number | null;
  onEdgeClick: (edgeId: number) => void;
  onNodeDoubleClick: (nodeId: number) => void;
  onNodeSelect: (nodeId: number) => void;
  positions?: PosicaoGrafo[];
  onPositionsChange?: (positions: PosicaoGrafo[]) => void;
}
export interface GraphViewerHandle { exportPng: () => string | null }
interface Overlay { id: number; x: number; y: number; zoom: number; hp: number; color: string }

export const GraphViewer = forwardRef<GraphViewerHandle, GraphViewerProps>(function GraphViewer(props, ref) {
  const { graph, layout, focusedNodeId, positions = [] } = props;
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const callbacksRef = useRef(props);
  const structureRef = useRef("");
  const [overlays, setOverlays] = useState<Overlay[]>([]);

  useEffect(() => { callbacksRef.current = props; }, [props]);
  useImperativeHandle(ref, () => ({ exportPng: () => {
    const cy = cyRef.current;
    if (!cy) return null;
    const result = cy.png({ bg: "#FFFFFF", full: true, maxHeight: 1800, maxWidth: 2400, scale: 2 });
    return typeof result === "string" ? result : null;
  }}), []);

  // Keep one Cytoscape instance so data refreshes retain the viewport and selection.
  useEffect(() => {
    if (!containerRef.current) return;
    const cy = cytoscape({ container: containerRef.current, elements: [], style: graphStyles,
      minZoom: 0.2, maxZoom: 3, wheelSensitivity: 0.18, boxSelectionEnabled: false });
    cyRef.current = cy;
    let frame = 0;
    const syncOverlays = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const zoom = cy.zoom();
        const next = cy.nodes().map((item) => {
          const node = item.data("profile") as GrafoNode;
          const pos = item.renderedPosition();
          return { id: node.id, x: pos.x, y: pos.y + item.renderedOuterHeight() / 2 + 32 * zoom, zoom, hp: node.hp, color: node.vitalidade_cor_hex };
        });
        setOverlays((previous) => JSON.stringify(previous) === JSON.stringify(next) ? previous : next);
      });
    };
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
    cy.on("dragfree", "node", () => callbacksRef.current.onPositionsChange?.(cy.nodes().map((node) => ({
      pessoa_id: Number(node.data("nodeId")), x: node.position("x"), y: node.position("y"),
    }))));
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
    const observer = new ResizeObserver(() => { cy.resize(); syncOverlays(); });
    observer.observe(containerRef.current);
    return () => {
      window.clearInterval(timer); cancelAnimationFrame(frame);
      motion.removeEventListener("change", applyMotion); observer.disconnect();
      cy.destroy(); cyRef.current = null; structureRef.current = "";
    };
  }, []);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    const elements: ElementDefinition[] = [
      ...graph.nodes.map((node) => ({ data: { id: `node-${node.id}`, nodeId: node.id,
        label: `${node.label} · HP ${node.hp_percentual.toLocaleString("pt-BR", { maximumFractionDigits: 2 })}%`,
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
    const structure = `${layout}:${graph.nodes.map((node) => node.id).sort((a, b) => a - b).join(",")}:${graph.edges.map((edge) => `${edge.id}-${edge.source}-${edge.target}`).sort().join(",")}`;
    if (structure !== structureRef.current) {
      structureRef.current = structure;
      const roots = hierarchicalRoots(graph);
      cy.layout(layout === "force" ? { name: "cose", animate: false, fit: true, padding: 90,
        idealEdgeLength: 155, nodeRepulsion: 8000, gravity: 0.22, numIter: 1200 }
        : { name: "breadthfirst", directed: true, fit: true, padding: 90, spacingFactor: 1.65,
          avoidOverlap: true, roots: roots.length ? roots : undefined }).run();
      for (const saved of callbacksRef.current.positions ?? []) {
        const node = cy.$id(`node-${saved.pessoa_id}`);
        if (!node.empty()) node.position({ x: saved.x, y: saved.y });
      }
    }
    // Highlight source profiles and the selected node's first/second degree neighborhood.
    cy.batch(() => {
      cy.elements().removeClass("is-source is-direct is-secondary focus-edge second-edge");
      if (focusedNodeId === null) return;
      const focus = cy.$id(`node-${focusedNodeId}`);
      if (focus.empty()) return;
      const direct = focus.neighborhood("node");
      const secondary = direct.neighborhood("node").difference(direct).difference(focus);
      direct.addClass("is-direct"); secondary.addClass("is-secondary");
      focus.connectedEdges().addClass("focus-edge");
      direct.connectedEdges().difference(focus.connectedEdges()).addClass("second-edge");
      const data = focus.data("profile") as GrafoNode;
      for (const row of data.contribuicoes) cy.$id(`node-${row.fonte_id}`).addClass("is-source");
    });
  }, [graph, layout, focusedNodeId]);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    for (const saved of positions) {
      const node = cy.$id(`node-${saved.pessoa_id}`);
      if (!node.empty()) node.position({ x: saved.x, y: saved.y });
    }
  }, [positions]);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy || focusedNodeId === null) return;
    const node = cy.$id(`node-${focusedNodeId}`);
    if (node.empty()) return;
    cy.nodes().unselect(); node.select();
    // Selecting a node highlights it without moving an already adjusted viewport.
  }, [focusedNodeId]);

  return <div className="relative h-full min-h-[38rem] w-full overflow-hidden">
    <div ref={containerRef} className="h-full min-h-[38rem] w-full cursor-grab active:cursor-grabbing" role="application" aria-label="Grafo interativo de relacionamentos" />
    <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
      {overlays.map(({ id, x, y, zoom, hp, color }) => <div key={id} className="absolute w-28" style={{ left: x, top: y, transform: `translateX(-50%) scale(${zoom})`, transformOrigin: "top center" }}>
        <VitalityBar hp={hp} color={color} compact />
      </div>)}
    </div>
  </div>;
});

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
    selector: "edge.is-hover, edge:selected",
    style: {
      width: 4,
      "line-color": "#E7654F",
      "target-arrow-color": "#E7654F",
      color: "#B83E2D",
    },
  },
];
