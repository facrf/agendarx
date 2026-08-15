import cytoscape from "cytoscape";
import type { Core, ElementDefinition, StylesheetJson } from "cytoscape";
import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { apiUrl } from "../services/api";
import type { GrafoResponse } from "../types/api";

export type GraphLayout = "force" | "hierarchical";

interface GraphViewerProps {
  graph: GrafoResponse;
  layout: GraphLayout;
  focusedNodeId: number | null;
  onEdgeClick: (edgeId: number) => void;
  onNodeDoubleClick: (nodeId: number) => void;
}

export interface GraphViewerHandle {
  exportPng: () => string | null;
}

export const GraphViewer = forwardRef<GraphViewerHandle, GraphViewerProps>(function GraphViewer({
  graph,
  layout,
  focusedNodeId,
  onEdgeClick,
  onNodeDoubleClick,
}, ref) {
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);

  useImperativeHandle(ref, () => ({
    exportPng: () => {
      const cy = cyRef.current;
      if (!cy) return null;
      const image = cy.png({
        bg: "#FFFFFF",
        full: true,
        maxHeight: 1_800,
        maxWidth: 2_400,
        scale: 2,
      });
      return typeof image === "string" ? image : null;
    },
  }), []);

  useEffect(() => {
    if (!containerRef.current) return;

    const elements: ElementDefinition[] = [
      ...graph.nodes.map((node) => ({
        data: {
          id: `node-${node.id}`,
          nodeId: node.id,
          label: node.label,
          color: node.color || "#86A6A3",
          image: node.foto_url ? apiUrl(node.foto_url) : "none",
          category: node.categoria || "Sem categoria",
          legalEntity: node.pessoa_juridica,
        },
      })),
      ...graph.edges.map((edge) => ({
        data: {
          id: `edge-${edge.id}`,
          edgeId: edge.id,
          source: `node-${edge.source}`,
          target: `node-${edge.target}`,
          label: edge.label,
        },
      })),
    ];

    const cy = cytoscape({
      container: containerRef.current,
      elements,
      style: graphStyles,
      minZoom: 0.2,
      maxZoom: 3,
      wheelSensitivity: 0.18,
      autoungrabify: false,
      userPanningEnabled: true,
      userZoomingEnabled: true,
      boxSelectionEnabled: false,
    });
    cyRef.current = cy;

    const rootIds = hierarchicalRoots(graph);
    const layoutRunner = cy.layout(
      layout === "force"
        ? {
            name: "cose",
            animate: false,
            fit: true,
            padding: 70,
            idealEdgeLength: 135,
            nodeRepulsion: 7200,
            gravity: 0.22,
            numIter: 1_200,
          }
        : {
            name: "breadthfirst",
            directed: true,
            circle: false,
            fit: true,
            padding: 70,
            spacingFactor: 1.45,
            avoidOverlap: true,
            roots: rootIds.length > 0 ? rootIds : undefined,
          },
    );
    layoutRunner.run();

    const focusNode = () => {
      if (focusedNodeId === null) return;
      const node = cy.$id(`node-${focusedNodeId}`);
      if (node.empty()) return;
      node.select();
      cy.animate({
        fit: { eles: node.closedNeighborhood(), padding: 110 },
        duration: 450,
        easing: "ease-out-cubic",
      });
    };
    window.setTimeout(focusNode, 60);

    let lastTap = { id: "", at: 0 };
    cy.on("tap", "edge", (event) => onEdgeClick(Number(event.target.data("edgeId"))));
    cy.on("tap", "node", (event) => {
      const id = String(event.target.id());
      const now = Date.now();
      if (lastTap.id === id && now - lastTap.at < 350) {
        onNodeDoubleClick(Number(event.target.data("nodeId")));
        lastTap = { id: "", at: 0 };
      } else {
        lastTap = { id, at: now };
      }
    });

    const observer = new ResizeObserver(() => cy.resize());
    observer.observe(containerRef.current);

    return () => {
      observer.disconnect();
      cy.destroy();
      cyRef.current = null;
    };
  }, [graph, layout, focusedNodeId, onEdgeClick, onNodeDoubleClick]);

  return (
    <div
      ref={containerRef}
      className="h-full min-h-[38rem] w-full cursor-grab active:cursor-grabbing"
      role="application"
      aria-label="Grafo interativo de relacionamentos"
    />
  );
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
    selector: "node[?legalEntity]",
    style: {
      shape: "round-rectangle",
    },
  },
  {
    selector: "node:hover",
    style: { width: 76, height: 76, "border-width": 7 },
  },
  {
    selector: "node:selected",
    style: {
      width: 78,
      height: 78,
      "border-color": "#E7654F",
      "border-width": 7,
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
    selector: "edge:hover, edge:selected",
    style: {
      width: 4,
      "line-color": "#E7654F",
      "target-arrow-color": "#E7654F",
      color: "#B83E2D",
    },
  },
];
