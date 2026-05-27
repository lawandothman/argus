export interface ServiceNode {
  id: string;
  label: string;
  color: string;
}

export interface ServiceEdge {
  from: string;
  to: string;
}

/** The checkout topology, mirroring the synthetic generator's call tree. */
export const SERVICES: ServiceNode[] = [
  { id: "api-gateway", label: "api-gateway", color: "#5eead4" },
  { id: "auth", label: "auth", color: "#a78bfa" },
  { id: "catalog", label: "catalog", color: "#60a5fa" },
  { id: "cart", label: "cart", color: "#34d399" },
  { id: "payments", label: "payments", color: "#fbbf24" },
  { id: "postgres", label: "postgres", color: "#f472b6" },
];

export const EDGES: ServiceEdge[] = [
  { from: "api-gateway", to: "auth" },
  { from: "api-gateway", to: "catalog" },
  { from: "api-gateway", to: "cart" },
  { from: "cart", to: "payments" },
  { from: "payments", to: "postgres" },
];

/** Entry point of the call tree. */
export const ROOT_ID = "api-gateway";

const byId = new Map(SERVICES.map((service) => [service.id, service]));

export const serviceById = (id: string): ServiceNode | undefined => byId.get(id);

/** The services called directly by `id`, in declaration order. */
export const childrenOf = (id: string): string[] =>
  EDGES.filter((edge) => edge.from === id).map((edge) => edge.to);
