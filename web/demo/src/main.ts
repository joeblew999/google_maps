import { createMapsClient } from "@joeblew999/google-maps-connect";

const $ = (id: string) => document.getElementById(id) as HTMLInputElement;
const out = document.getElementById("out") as HTMLPreElement;
const client = () => createMapsClient($("url").value, $("token").value || undefined);

async function run(call: () => Promise<unknown>) {
  out.textContent = "calling…";
  try {
    // int64 proto fields (e.g. durationSeconds) come back as JS BigInt, which
    // JSON.stringify can't serialize — render them as strings.
    out.textContent = JSON.stringify(
      await call(),
      (_k, v) => (typeof v === "bigint" ? v.toString() : v),
      2,
    );
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
on("distancematrix", () =>
  client().distanceMatrix({
    origins: [{ latitude: +$("dmolat").value, longitude: +$("dmolng").value }],
    destinations: [{ latitude: +$("dmdlat").value, longitude: +$("dmdlng").value }],
  }));
on("autocomplete", () => client().placesAutocomplete({ input: $("acinput").value }));
on("nearby", () =>
  client().placesNearby({
    latitude: +$("nblat").value,
    longitude: +$("nblng").value,
    radiusMeters: +$("nbradius").value,
  }));
on("placedetails", () => client().placeDetails({ placeId: $("pdid").value }));
on("placephotos", () =>
  client().placePhotos({ photoName: $("ppname").value, maxWidthPx: +$("ppwidth").value }));
on("snaptoroads", () =>
  client().snapToRoads({
    path: [
      { latitude: +$("srlat1").value, longitude: +$("srlng1").value },
      { latitude: +$("srlat2").value, longitude: +$("srlng2").value },
    ],
    interpolate: ($("srinterp") as HTMLInputElement).checked,
  }));
on("nearestroads", () =>
  client().nearestRoads({
    points: [
      { latitude: +$("nrlat1").value, longitude: +$("nrlng1").value },
      { latitude: +$("nrlat2").value, longitude: +$("nrlng2").value },
    ],
  }));
on("validateaddress", () =>
  client().validateAddress({
    addressLines: [$("valines").value],
    regionCode: $("varegion").value,
  }));
