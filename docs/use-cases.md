# Uses Cases 



## Smart In-App Search & Auto-Fill

Location-Aware Registration: Auto-complete addresses when coaches create events, instantly calculating and saving latitude, longitude, and local time zones.

Radius-Based Event Discovery: Let parents type their zip code into your app to instantly find all active leagues or clinics within a 15-mile radius.

## Logistical & Administrative Analytics

Travel Distance Calculators: Use the Distance Matrix API in your backend to automatically calculate total travel mileage for all teams to optimize tournament seeding and division grouping.

Centralized Status Flags: Mark fields as "Rainout" or "Delayed" in your database to instantly change the color of the embedded map pin from green to red for all users simultaneously.

Geofenced Attendance Check-ins: Cross-reference a student’s live device location with the stadium's boundary coordinates to automate attendance tracking for coaches when the player arrives. Same as Grab does.

## Primitives coverage

There are two kinds of primitive: **fetch** (the Connect RPCs the worker serves) and
**interaction/render** (an actual map, markers, type-ahead, browser geolocation). The maps
worker + clients cover the fetch side; the render side lives in the consuming app.

| Use case | Fetch primitive(s) | Status |
|---|---|---|
| Auto-fill registration (autocomplete → lat/lng + tz) | `PlacesAutocomplete` → `Geocode` → `TimeZone` | ✅ all served |
| Radius event discovery (zip → leagues within N mi) | `Geocode` the zip; radius filter is your DB (haversine). For Google POIs: `PlacesNearby` | ✅ served (radius filter is app-side) |
| Travel distance calculator (N×M mileage) | `DistanceMatrix` (numeric `distance_meters` / `duration_seconds`) | ✅ served. Inputs are coordinates — geocode addresses first. |
| Status-flag pin colors (Rainout → red pin) | none (your DB) + a rendered map | ⚠️ fetch side trivial; needs a **map render** primitive (not in the demo) |
| Geofenced check-in (device vs boundary) | `Geocode` stadium once; point-in-polygon is app-side; device pos = browser Geolocation | ⚠️ barely a Maps concern; lives in the app/browser |

**RPCs now available:** Geocode, ReverseGeocode, Directions (numeric distance/duration),
Elevation, TimeZone, TextSearch, DistanceMatrix, PlacesAutocomplete, PlacesNearby. `Place`
carries `latitude`/`longitude`/`place_id` so search/nearby results are directly plottable.

**Still UI, not RPC:** an interactive map with colored markers + a type-ahead box. The vanilla
`web/demo` is a JSON harness by design; a map demo (e.g. MapLibre) would be a separate, optional
build so the simple demo stays simple.