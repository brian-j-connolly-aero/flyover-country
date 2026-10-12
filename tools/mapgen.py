#!/usr/bin/env python3
"""Generate content/map.ron from real-world coordinates.

The map is a schematic West: real places, projected to integer map units and
then nudged apart so nothing overlaps on a TV across the room. Edit the tables
below and rerun:  python3 tools/mapgen.py > content/map.ron

Map units: x grows east, y grows south. 1 degree of latitude = 40 units.
"""
import math

LON0, LAT0 = -125.0, 49.5
KX, KY = 30.6, 40.0  # cos(40 deg) * 40 ~= 30.6


def proj(lat, lon):
    return ((lon - LON0) * KX, (LAT0 - lat) * KY)


# --- Zones (old state borders, simplified, shared vertices) -----------------
ZONES = [
    # id, name, region, [(lat, lon), ...]
    ("WA", "Washington", "Cascadia", [(48.4, -124.7), (49, -123.3), (49, -117.03), (46, -117.03), (46, -119.0), (45.65, -121.0), (45.6, -122.7), (46.25, -124.0)]),
    ("OR", "Oregon", "Cascadia", [(46.25, -124.0), (45.6, -122.7), (45.65, -121.0), (46, -119.0), (46, -117.03), (44.5, -117.2), (43.6, -117.03), (42, -117.03), (42, -120.0), (42, -124.2), (43.4, -124.3)]),
    ("CA", "California", "Pacific", [(42, -124.2), (42, -120.0), (39.0, -120.0), (35.0, -114.63), (34.3, -114.14), (32.72, -114.72), (32.53, -117.12), (34.0, -118.5), (34.45, -120.47), (36.3, -121.9), (37.8, -122.5), (40.4, -124.4)]),
    ("NV", "Nevada", "Dreamland", [(42, -120.0), (42, -117.03), (42, -114.05), (37, -114.05), (36.1, -114.05), (36.0, -114.74), (35.0, -114.63), (39.0, -120.0)]),
    ("ID", "Idaho", "BigEmpty", [(49, -117.03), (49, -116.05), (47.6, -115.7), (46.6, -114.5), (45.6, -114.5), (44.5, -112.8), (44.5, -111.05), (42, -111.05), (42, -114.05), (42, -117.03), (43.6, -117.03), (44.5, -117.2), (46, -117.03)]),
    ("MT", "Montana", "BigEmpty", [(49, -116.05), (49, -104.05), (45.94, -104.05), (45, -104.05), (45, -111.05), (44.5, -111.05), (44.5, -112.8), (45.6, -114.5), (46.6, -114.5), (47.6, -115.7)]),
    ("WY", "Wyoming", "BigEmpty", [(45, -111.05), (45, -104.05), (43, -104.05), (41, -104.05), (41, -109.05), (41, -111.05), (42, -111.05), (44.5, -111.05)]),
    ("UT", "Utah", "Beehive", [(42, -114.05), (42, -111.05), (41, -111.05), (41, -109.05), (37, -109.05), (37, -114.05)]),
    ("CO", "Colorado", "FrontRange", [(41, -109.05), (41, -104.05), (41, -102.05), (40, -102.05), (37, -102.05), (37, -103.0), (37, -109.05)]),
    ("AZ", "Arizona", "BigEmpty", [(37, -114.05), (37, -109.05), (31.33, -109.05), (31.33, -111.07), (32.49, -114.81), (32.72, -114.72), (34.3, -114.14), (35.0, -114.63), (36.0, -114.74), (36.1, -114.05)]),
    ("NM", "New Mexico", "BigEmpty", [(37, -109.05), (37, -103.0), (36.5, -103.0), (32.0, -103.06), (32.0, -106.6), (31.78, -106.53), (31.78, -108.2), (31.33, -108.2), (31.33, -109.05)]),
    ("TX", "Texas", "Texas", [(36.5, -103.0), (36.5, -100.0), (34.56, -100.0), (33.9, -98.0), (33.8, -96.6), (33.6, -94.04), (31.0, -93.55), (29.7, -93.84), (29.3, -94.8), (28.0, -97.0), (26.0, -97.2), (26.0, -98.0), (27.5, -99.5), (29.8, -101.4), (29.0, -103.1), (29.8, -104.6), (31.78, -106.53), (32.0, -106.6), (32.0, -103.06)]),
    ("OK", "Oklahoma", "GrainBelt", [(37, -103.0), (37, -102.05), (37, -94.62), (36.5, -94.62), (35.4, -94.43), (33.6, -94.04), (33.8, -96.6), (33.9, -98.0), (34.56, -100.0), (36.5, -100.0), (36.5, -103.0)]),
    ("KS", "Kansas", "GrainBelt", [(40, -102.05), (40, -95.3), (39.1, -94.6), (37, -94.62), (37, -102.05)]),
    ("NE", "Nebraska", "GrainBelt", [(43, -104.05), (43, -98.5), (42.8, -97.2), (42.5, -96.5), (41.3, -95.9), (40, -95.3), (40, -102.05), (41, -102.05), (41, -104.05)]),
    ("SD", "South Dakota", "GrainBelt", [(45.94, -104.05), (45.94, -96.56), (45.3, -96.45), (43.5, -96.45), (42.5, -96.5), (42.8, -97.2), (43, -98.5), (43, -104.05), (45, -104.05)]),
    ("ND", "North Dakota", "GrainBelt", [(49, -104.05), (49, -97.23), (45.94, -96.56), (45.94, -104.05)]),
]

# Zone centers tuned by hand where the centroid sits somewhere silly.
ZONE_CENTER_OVERRIDE = {
    "CA": (36.6, -119.6),
    "ID": (44.2, -114.9),
    "TX": (31.4, -99.3),
    "OK": (35.6, -97.4),
    "NV": (38.6, -116.6),
}

# Strip of old states between the Grain Belt and the river. No nodes; the
# Static creeps in from here.
FRINGE = [
    ("MN", [(49, -97.23), (49.3, -95.15), (48.0, -89.5), (46.8, -92.1), (45.6, -92.8), (44.6, -92.5), (43.5, -91.2), (43.5, -96.45), (45.3, -96.45), (45.94, -96.56)]),
    ("IA", [(43.5, -96.45), (43.5, -91.2), (42.5, -90.6), (41.5, -90.6), (40.6, -91.4), (40.6, -95.7), (41.3, -95.9), (42.5, -96.5)]),
    ("MO", [(40.6, -95.7), (40.6, -91.4), (39.0, -90.2), (37.0, -89.2), (36.0, -90.1), (36.5, -94.62), (37, -94.62), (39.1, -94.6), (40, -95.3)]),
    ("AR", [(36.5, -94.62), (36.0, -90.1), (34.9, -90.3), (33.0, -91.1), (33.0, -94.04), (33.6, -94.04), (35.4, -94.43)]),
    ("LA", [(33.0, -94.04), (33.0, -91.1), (31.0, -91.6), (31.0, -89.7), (29.1, -89.4), (29.5, -91.5), (29.7, -93.84), (31.0, -93.55)]),
]

# The Mississippi, north to south. East of it: the Static.
MISSISSIPPI = [(49.5, -89.3), (47.2, -91.0), (45.0, -92.8), (43.5, -91.2), (41.5, -90.6), (39.0, -90.2), (37.0, -89.2), (35.0, -90.1), (33.0, -91.1), (31.0, -91.6), (29.1, -89.4), (25.5, -89.4)]

# --- Sites: four per faction ----------------------------------------------------
# id, name, kind, faction, lat, lon, tagline
SITES = [
    ("pac_hq", "Palo Alto", "HQ", "Pacific", 37.44, -122.14, "Headquarters. Also a juice bar."),
    ("pac_air", "Fairfield Field", "Airfield", "Pacific", 38.6, -121.6, "Where the squadrons sleep."),
    ("pac_factory", "Mojave Works", "Factory", "Pacific", 35.05, -118.17, "Builds planes, ships them in beta."),
    ("pac_jewel", "The Last Server Farm", "Jewel", "Pacific", 36.6, -121.0, "The last one. Don't unplug it."),

    ("cas_hq", "Portland", "HQ", "Cascadia", 45.52, -122.68, "Keeps it weird. Keeps it the capital."),
    ("cas_air", "Moses Lake Field", "Airfield", "Cascadia", 47.13, -119.6, "A runway long enough to land a regret."),
    ("cas_factory", "Everett Works", "Factory", "Cascadia", 47.95, -122.2, "Wide-body planes, built slowly and well."),
    ("cas_jewel", "Grand Coulee Dam", "Jewel", "Cascadia", 47.96, -118.98, "Powers the Commonwealth. Mostly the coffee."),

    ("frs_hq", "Denver", "HQ", "FrontRange", 39.74, -104.99, "A mile up and proud of it."),
    ("frs_air", "Limon Field", "Airfield", "FrontRange", 39.27, -103.69, "Out on the plains, where the wind does the waving."),
    ("frs_factory", "Grand Junction Works", "Factory", "FrontRange", 39.06, -108.55, "Peaches, canyons and fighter planes."),
    ("frs_jewel", "Cheyenne Mountain", "Jewel", "FrontRange", 38.74, -104.85, "A whole mountain of radar."),

    ("tex_hq", "Austin", "HQ", "Texas", 30.27, -97.74, "Weird on purpose."),
    ("tex_air", "Abilene Field", "Airfield", "Texas", 32.42, -99.85, "Home of the big bombers."),
    ("tex_factory", "Fort Worth Works", "Factory", "Texas", 32.75, -97.33, "Where the big bombers get built."),
    ("tex_jewel", "The Gulf Refinery", "Jewel", "Texas", 29.74, -95.01, "Flares visible from space."),

    ("drm_hq", "Las Vegas", "HQ", "Dreamland", 36.17, -115.14, "The house. The house always wins."),
    ("drm_air", "Tonopah Field", "Airfield", "Dreamland", 38.07, -117.08, "Officially, nothing is here."),
    ("drm_factory", "Groom Lake Works", "Factory", "Dreamland", 37.4, -115.6, "Officially, nothing is built here."),
    ("drm_jewel", "Hoover Dam", "Jewel", "Dreamland", 35.35, -114.95, "Keeps the Strip lit."),

    ("grb_hq", "Lincoln", "HQ", "GrainBelt", 40.81, -96.7, "The capital of the breadbasket."),
    ("grb_air", "Hays Field", "Airfield", "GrainBelt", 38.88, -99.33, "Flat enough to land anywhere."),
    ("grb_factory", "Wichita Works", "Factory", "GrainBelt", 37.69, -97.34, "The Air Capital, still."),
    ("grb_jewel", "World's Largest Ball of Twine", "Jewel", "GrainBelt", 39.51, -98.43, "Still growing."),

    ("bee_hq", "Salt Lake City", "HQ", "Beehive", 40.76, -111.89, "Tidy, polite and well organized."),
    ("bee_air", "Wendover Field", "Airfield", "Beehive", 40.73, -114.03, "Half in Utah, half in a casino."),
    ("bee_factory", "St. George Works", "Factory", "Beehive", 37.2, -113.58, "Builds planes and casseroles."),
    ("bee_jewel", "Strategic Fry Sauce Reserve", "Jewel", "Beehive", 39.35, -111.6, "Pink, sacred, strategic."),
]

# Prizes in the Big Empty: whoever holds one at dawn scores and gets Supply.
PRIZES = [
    ("prize_faithful", "Old Faithful", 44.46, -110.83, "Erupts on schedule. Nothing else does."),
    ("prize_boneyard", "The Boneyard", 32.17, -110.85, "Thousands of old planes, some of them still willing."),
    ("prize_whitesands", "White Sands", 32.9, -106.4, "Endless white dunes and a half-built space gun."),
]

# Places on the map that aren't targets.
LANDMARKS = [
    ("bark_park", "Project Bark Park", (36.999, -109.045),
     "Four Corners. A secret dog park with one very large tennis-ball launcher."),
]

# --- Projection: sites keep their real places, nudged apart for labels ---------
MIN_SPACING = 30.0
P = []
for sid, name, kind, fac, lat, lon, tag in SITES:
    x, y = proj(lat, lon)
    P.append(dict(id=sid, name=name, kind=kind, faction=fac, x=x, y=y, x0=x, y0=y, tag=tag))
for _ in range(300):
    moved = False
    for i in range(len(P)):
        for j in range(i + 1, len(P)):
            a, b = P[i], P[j]
            dx, dy = b["x"] - a["x"], b["y"] - a["y"]
            d = math.hypot(dx, dy)
            if d < MIN_SPACING:
                moved = True
                if d < 1e-6:
                    dx, dy, d = 1.0, 0.0, 1.0
                push = (MIN_SPACING - d) / 2 + 0.05
                ux, uy = dx / d, dy / d
                a["x"] -= ux * push
                a["y"] -= uy * push
                b["x"] += ux * push
                b["y"] += uy * push
    if not moved:
        break


def pip(px, py, poly):
    inside = False
    j = len(poly) - 1
    for i in range(len(poly)):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if (yi > py) != (yj > py):
            xin = (xj - xi) * (py - yi) / (yj - yi) + xi
            if px < xin:
                inside = not inside
        j = i
    return inside


zone_polys = {}
for zid, name, region, pts in ZONES:
    zone_polys[zid] = [proj(lat, lon) for lat, lon in pts]
REGION_ZONES = {}
for zid, name, region, pts in ZONES:
    REGION_ZONES.setdefault(region, []).append(zid)


def region_of(x, y):
    for zid, name, region, pts in ZONES:
        if pip(x, y, zone_polys[zid]):
            return region
    return None


for s in P:
    assert region_of(s["x"], s["y"]) == s["faction"], (s["id"], region_of(s["x"], s["y"]))
for pid, name, lat, lon, tag in PRIZES:
    assert region_of(*proj(lat, lon)) == "BigEmpty", pid


def centroid(poly):
    a = cx = cy = 0.0
    for i in range(len(poly)):
        x0, y0 = poly[i]
        x1, y1 = poly[(i + 1) % len(poly)]
        c = x0 * y1 - x1 * y0
        a += c
        cx += (x0 + x1) * c
        cy += (y0 + y1) * c
    a *= 0.5
    return cx / (6 * a), cy / (6 * a)


def ipt(p):
    return "(%d, %d)" % (round(p[0]), round(p[1]))


def q(s):
    return s.replace('"', '\\"')


out = []
out.append("// Generated by tools/mapgen.py. Edit the script, not this file.")
out.append("// Map units: x grows east, y grows south; 1 degree of latitude = 40 units.")
out.append("MapDef(")
out.append("    width: %d," % round(proj(25.5, -89.0)[0]))
out.append("    height: %d," % round(proj(25.5, -89.0)[1]))
out.append("    zones: [")
for zid, name, region, pts in ZONES:
    poly = zone_polys[zid]
    c = ZONE_CENTER_OVERRIDE.get(zid)
    c = proj(*c) if c else centroid(poly)
    out.append("        ZoneDef(id: \"%s\", name: \"%s\", region: %s, center: %s, poly: [%s])," % (
        zid, name, region, ipt(c), ", ".join(ipt(p) for p in poly)))
out.append("    ],")
out.append("    fringe: [")
for zid, pts in FRINGE:
    out.append("        [%s], // %s" % (", ".join(ipt(proj(lat, lon)) for lat, lon in pts), zid))
out.append("    ],")
out.append("    mississippi: [%s]," % ", ".join(ipt(proj(lat, lon)) for lat, lon in MISSISSIPPI))
out.append("    sites: [")
for s in P:
    out.append("        SiteDef(id: \"%s\", name: \"%s\", kind: %s, faction: %s, pos: %s, tagline: \"%s\")," % (
        s["id"], q(s["name"]), s["kind"], s["faction"], ipt((s["x"], s["y"])), q(s["tag"])))
out.append("    ],")
out.append("    prizes: [")
for pid, name, lat, lon, tag in PRIZES:
    out.append("        PrizeDef(id: \"%s\", name: \"%s\", pos: %s, tagline: \"%s\")," % (pid, q(name), ipt(proj(lat, lon)), q(tag)))
out.append("    ],")
out.append("    landmarks: [")
for lid, name, (lat, lon), tag in LANDMARKS:
    out.append("        LandmarkDef(id: \"%s\", name: \"%s\", pos: %s, tagline: \"%s\")," % (
        lid, q(name), ipt(proj(lat, lon)), q(tag)))
out.append("    ],")
out.append(")")
print("\n".join(out))
