// Reusable, framework-agnostic typed client for maps.v1.MapsService.
// No React, no Kumo — just the generated types + a one-call client factory.
// Works in the browser, Node, or another Worker; point it at any base URL.
import { createClient, type Client } from "@connectrpc/connect";
import { createConnectTransport } from "@connectrpc/connect-web";
import { MapsService } from "./gen/maps/v1/maps_pb.js";

export * from "./gen/maps/v1/maps_pb.js";
export { MapsService };

export type MapsClient = Client<typeof MapsService>;

/**
 * Create a typed Maps client.
 * @param baseUrl - the shared maps worker, or a project worker that composes it.
 */
export function createMapsClient(baseUrl: string): MapsClient {
  return createClient(MapsService, createConnectTransport({ baseUrl }));
}
