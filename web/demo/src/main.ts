import { createMapsClient } from "@joeblew999/google-maps-connect";

const $ = (id: string) => document.getElementById(id) as HTMLInputElement;
const out = document.getElementById("out") as HTMLPreElement;

document.getElementById("go")!.addEventListener("click", async () => {
  out.textContent = "calling…";
  try {
    const client = createMapsClient($("url").value);
    const res = await client.geocode({ address: $("q").value });
    out.textContent = JSON.stringify(res, null, 2);
  } catch (e) {
    out.textContent = "error: " + (e as Error).message;
  }
});
