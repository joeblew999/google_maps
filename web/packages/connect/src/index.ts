// Reusable, framework-agnostic typed client for maps.v1.MapsService.
// No React, no Kumo — just the generated types + a one-call client factory.
// Works in the browser, Node, or another Worker; point it at any base URL.
import { createClient, type Client, type Interceptor } from "@connectrpc/connect";
import { createConnectTransport } from "@connectrpc/connect-web";
import { MapsService } from "./gen/maps/v1/maps_pb.js";

export * from "./gen/maps/v1/maps_pb.js";
export { MapsService };

export type MapsClient = Client<typeof MapsService>;

/** Adds `Authorization: Bearer <token>` to every request. */
const bearer = (token: string): Interceptor => (next) => (req) => {
  req.header.set("Authorization", `Bearer ${token}`);
  return next(req);
};

/**
 * Create a typed Maps client.
 * @param baseUrl - the shared maps worker, or a project worker that composes it.
 * @param token - optional Bearer token for the worker's token gate. In production
 *   the browser should NOT hold this — call a project worker that injects it
 *   server-side. Passing it here is for local/dev testing against a gated worker.
 */
export function createMapsClient(baseUrl: string, token?: string): MapsClient {
  const interceptors = token ? [bearer(token)] : [];
  return createClient(MapsService, createConnectTransport({ baseUrl, interceptors }));
}
