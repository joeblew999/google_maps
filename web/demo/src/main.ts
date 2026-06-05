import { createMapsClient } from "@joeblew999/google-maps-connect";

const $ = (id: string) => document.getElementById(id) as HTMLInputElement;
const out = document.getElementById("out") as HTMLPreElement;
const client = () => createMapsClient($("url").value);

async function run(call: () => Promise<unknown>) {
  out.textContent = "calling…";
  try {
    out.textContent = JSON.stringify(await call(), null, 2);
  } catch (e) {
    out.textContent = "error: " + (e as Error).message;
  }
}

const on = (id: string, call: () => Promise<unknown>) =>
  document.getElementById(id)!.addEventListener("click", () => run(call));

on("geocode", () => client().geocode({ address: $("addr").value }));
on("reverse", () =>
  client().reverseGeocode({ latitude: +$("lat").value, longitude: +$("lng").value }));
on("directions", () =>
  client().directions({ origin: $("origin").value, destination: $("destination").value }));
on("textsearch", () => client().textSearch({ query: $("query").value }));
on("elevation", () => client().elevation({ latitude: +$("elat").value, longitude: +$("elng").value }));
on("timezone", () => client().timeZone({ latitude: +$("tzlat").value, longitude: +$("tzlng").value }));
