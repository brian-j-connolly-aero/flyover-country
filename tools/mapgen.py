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

# --- Nodes ---------------------------------------------------------------------
# id, name, kind, owner, lat, lon, objective, tagline
N = []


def node(nid, name, kind, owner, lat, lon, objective=0, tag=""):
    N.append(dict(id=nid, name=name, kind=kind, owner=owner, lat=lat, lon=lon, objective=objective, tag=tag))


# Cascadian Commonwealth
node("cas_capital", "Portland", "Capital", "Cascadia", 45.52, -122.68, tag="Keeps it weird. Keeps it the capital.")
node("cas_seattle", "Seattle", "City", "Cascadia", 47.61, -122.33, tag="Coffee, ferries and a very patient population.")
node("cas_spokane", "Spokane", "City", "Cascadia", 47.66, -117.43, tag="The dry side. Doesn't like to talk about it.")
node("cas_refinery", "Cherry Point Refinery", "Fuel", "Cascadia", 48.86, -122.75, tag="Fuel, with a view of three volcanoes.")
node("cas_factory", "Tri-Cities Works", "Factory", "Cascadia", 46.25, -119.2, tag="Builds squadrons after a thorough public comment period.")
node("cas_ab_moses", "Moses Lake Field", "Airbase", "Cascadia", 47.13, -119.28, tag="A runway long enough to land a regret.")
node("cas_ab_klamath", "Klamath Field", "Airbase", "Cascadia", 42.2, -121.73, tag="The southern door, usually under cloud.")
node("cas_radar", "Mount Rainier Radar", "Radar", "Cascadia", 46.85, -121.76, tag="Sees a hundred miles on the one clear day a year.")
node("cas_ad_puget", "Puget Sound Battery", "AirDefense", "Cascadia", 47.2, -122.95, tag="Guns tucked between the ferries.")
node("cas_ad_gorge", "Columbia Gorge Battery", "AirDefense", "Cascadia", 45.65, -121.45, tag="The wind does half the work.")
node("cas_jct_yakima", "Yakima Junction", "Junction", "Cascadia", 46.6, -120.5, tag="Hops, apples and every train in the state.")
node("cas_jct_bend", "Bend Junction", "Junction", "Cascadia", 44.06, -121.31, tag="Where the highways meet the microbreweries.")
node("cas_jewel", "Grand Coulee Dam", "CrownJewel", "Cascadia", 47.96, -118.98, tag="Powers every factory in Cascadia.")
node("lake_puget", "Puget Sound", "Lake", "Cascadia", 48.25, -122.55, tag="Floatplane country.")
node("lake_crater", "Crater Lake", "Lake", "Cascadia", 42.94, -122.11, tag="Deep, blue, and full of floatplanes.")

# Pacific Venture Republic
node("pac_capital", "Palo Alto", "Capital", "Pacific", 37.44, -122.14, tag="Capital, HQ and incubator, all in one garage.")
node("pac_la", "Los Angeles", "City", "Pacific", 34.05, -118.24, tag="Eighteen million people and one free parking space.")
node("pac_sd", "San Diego", "City", "Pacific", 32.72, -117.16, tag="Perfect weather, as a service.")
node("pac_sac", "Sacramento", "City", "Pacific", 38.58, -121.49, tag="The other capital. Still a little hurt about it.")
node("pac_oil", "Bakersfield Oil Fields", "Fuel", "Pacific", 35.37, -119.02, tag="Old-school fuel for a new-school nation.")
node("pac_factory", "Mojave Works", "Factory", "Pacific", 35.05, -118.17, tag="Ships drones fast. Fixes them never.")
node("pac_ab_fairfield", "Fairfield Field", "Airbase", "Pacific", 38.26, -121.93, tag="Jelly beans and jet wash.")
node("pac_ab_29", "Twentynine Palms Field", "Airbase", "Pacific", 34.24, -116.05, tag="Hot, dry and extremely online.")
node("pac_ab_redding", "Redding Field", "Airbase", "Pacific", 40.58, -122.39, tag="The northern frontier of the startup scene.")
node("pac_radar_vandenberg", "Vandenberg Radar", "Radar", "Pacific", 34.74, -120.57, tag="Watching the coast for disruptors.")
node("pac_radar_shasta", "Mount Shasta Radar", "Radar", "Pacific", 41.41, -122.19, tag="Keeps an eye on the rain people.")
node("pac_ad_diablo", "Mount Diablo Battery", "AirDefense", "Pacific", 37.88, -121.7, tag="Fog-rated air defense with a view of the Bay.")
node("pac_ad_ie", "Inland Empire Battery", "AirDefense", "Pacific", 34.06, -117.3, tag="Covers the warehouses and the traffic.")
node("pac_jct_fresno", "Fresno Junction", "Junction", "Pacific", 36.74, -119.79, tag="Everything north-south goes through here.")
node("pac_jct_barstow", "Barstow Junction", "Junction", "Pacific", 34.9, -117.02, tag="Rail yard, truck stop, spiritual crossroads.")
node("pac_jewel", "The Last Server Farm", "CrownJewel", "Pacific", 37.05, -121.6, tag="Runs every drone. Smells faintly of garlic.")

# Dreamland Syndicate
node("drm_capital", "Las Vegas", "Capital", "Dreamland", 36.17, -115.14, tag="The Strip never sleeps, it just dims.")
node("drm_reno", "Reno", "City", "Dreamland", 39.53, -119.81, tag="The biggest little city, open late.")
node("drm_laughlin", "Laughlin", "City", "Dreamland", 35.17, -114.57, tag="River casinos and riverboat odds.")
node("drm_oil", "Railroad Valley Oil", "Fuel", "Dreamland", 38.5, -115.7, tag="Nobody remembers drilling here. Nobody asks.")
node("drm_factory", "Groom Lake Works", "Factory", "Dreamland", 37.24, -115.81, tag="Officially a parking lot.")
node("drm_ab_tonopah", "Tonopah Field", "Airbase", "Dreamland", 38.07, -117.08, tag="Middle of nowhere, on purpose.")
node("drm_ab_fallon", "Fallon Field", "Airbase", "Dreamland", 39.42, -118.7, tag="Top fighters, cheap buffets.")
node("drm_radar_ely", "Ely Radar", "Radar", "Dreamland", 39.25, -114.89, tag="Sees all the way to Utah. Doesn't gossip.")
node("drm_radar_beatty", "Beatty Radar", "Radar", "Dreamland", 36.9, -116.76, tag="Pointed at the sky, and occasionally at saucers.")
node("drm_ad_boulder", "Boulder City Battery", "AirDefense", "Dreamland", 35.9, -114.95, tag="Guards the dam that guards the Strip.")
node("drm_ad_winnemucca", "Winnemucca Battery", "AirDefense", "Dreamland", 40.97, -117.74, tag="Covers the northern approach and a great diner.")
node("drm_jct_hawthorne", "Hawthorne Junction", "Junction", "Dreamland", 38.52, -118.62, tag="Two highways and a slot machine.")
node("drm_jct_caliente", "Caliente Junction", "Junction", "Dreamland", 37.61, -114.51, tag="The rail line to everywhere else.")
node("drm_jewel", "Hoover Dam", "CrownJewel", "Dreamland", 36.2, -114.6, tag="Every neon light on the Strip runs through here.")
node("lake_tahoe", "Lake Tahoe", "Lake", None, 39.1, -120.03, tag="Two states, one lake, zero agreement.")
node("lake_mead", "Lake Mead", "Lake", None, 36.25, -114.35, tag="The bathtub ring of the West.")

# Beehive Cooperative
node("bee_capital", "Salt Lake City", "Capital", "Beehive", 40.76, -111.89, tag="Clean streets, wide streets, polite streets.")
node("bee_provo", "Provo", "City", "Beehive", 40.23, -111.66, tag="Sensible sweaters and remarkable efficiency.")
node("bee_stgeorge", "St. George", "City", "Beehive", 37.1, -113.58, tag="Red rock, golf and a lot of minivans.")
node("bee_oil", "Uinta Basin Oil", "Fuel", "Beehive", 40.3, -109.8, tag="Fuel, dinosaurs and very few people.")
node("bee_factory", "Ogden Works", "Factory", "Beehive", 41.22, -111.97, tag="Industry is the state motto. They mean it.")
node("bee_ab_wendover", "Wendover Field", "Airbase", "Beehive", 40.73, -114.03, tag="Half in Utah, half in a casino.")
node("bee_ab_moab", "Moab Field", "Airbase", "Beehive", 38.57, -109.55, tag="Arches on the left, runway on the right.")
node("bee_radar", "Brian Head Radar", "Radar", "Beehive", 37.69, -112.85, tag="A ski resort that also watches the skies.")
node("bee_ad_wasatch", "Wasatch Battery", "AirDefense", "Beehive", 40.45, -111.3, tag="The Greatest Snow on Earth, plus guns.")
node("bee_ad_richfield", "Richfield Battery", "AirDefense", "Beehive", 38.77, -112.08, tag="Watches I-70 and the pie shop.")
node("bee_jct_spike", "Golden Spike Junction", "Junction", "Beehive", 41.62, -112.55, tag="Where the railroads met. They still meet here.")
node("bee_jct_green", "Green River Junction", "Junction", "Beehive", 38.99, -110.16, tag="Melons, rail and a long way to anywhere.")
node("bee_jewel", "Strategic Fry Sauce Reserve", "CrownJewel", "Beehive", 40.5, -112.4, tag="Pink, beloved at every dinner, and stored in bulk.")
node("lake_salt", "Great Salt Lake", "Lake", None, 41.15, -112.6, tag="Floats anything. Including floatplanes.")

# Front Range Free State
node("frs_capital", "Denver", "Capital", "FrontRange", 39.74, -104.99, tag="Run from deep inside the mountain, a mile up.")
node("frs_springs", "Colorado Springs", "City", "FrontRange", 38.9, -104.6, tag="Garden of the Gods, parking of the mortals.")
node("frs_gj", "Grand Junction", "City", "FrontRange", 39.06, -108.55, tag="Peaches and the western slope.")
node("frs_brew_fc", "Fort Collins Brewery", "Fuel", "FrontRange", 40.59, -105.08, tag="Jet fuel, dry-hopped.")
node("frs_brew_steamboat", "Steamboat Brewery", "Fuel", "FrontRange", 40.48, -106.83, tag="Brewed with snowmelt and stubbornness.")
node("frs_brew_durango", "Durango Brewery", "Fuel", "FrontRange", 37.27, -107.88, tag="Narrow gauge, wide hops.")
node("frs_brew_alamosa", "Alamosa Brewery", "Fuel", "FrontRange", 37.47, -105.87, tag="Sand dunes and a hazy IPA.")
node("frs_factory", "Pueblo Works", "Factory", "FrontRange", 38.25, -104.61, tag="Steel town. Builds the planes, roasts the chiles.")
node("frs_ab_limon", "Limon Field", "Airbase", "FrontRange", 39.27, -103.69, tag="Flat enough to see tomorrow.")
node("frs_ab_gunnison", "Gunnison Field", "Airbase", "FrontRange", 38.55, -106.93, tag="Coldest runway in the West. Proudly.")
node("frs_radar_pikes", "Pikes Peak Radar", "Radar", "FrontRange", 38.84, -105.2, tag="Purple mountain majesty, with a dish on top.")
node("frs_radar_elbert", "Mount Elbert Radar", "Radar", "FrontRange", 39.12, -106.45, tag="Highest radar in the West. Brings snacks.")
node("frs_ad_loveland", "Loveland Pass Battery", "AirDefense", "FrontRange", 39.66, -105.88, tag="Covers the tunnel and the ski traffic.")
node("frs_ad_raton", "Raton Pass Battery", "AirDefense", "FrontRange", 37.05, -104.5, tag="The southern gate.")
node("frs_jct_glenwood", "Glenwood Junction", "Junction", "FrontRange", 39.55, -107.32, tag="Hot springs and the only way through the canyon.")
node("frs_jct_julesburg", "Julesburg Junction", "Junction", "FrontRange", 40.99, -102.26, tag="Rail corner of the Free State.")
node("frs_jewel", "Cheyenne Mountain", "CrownJewel", "FrontRange", 38.6, -104.95, tag="Can't be destroyed. Can be very annoyed.")

# Republic of Texas (Again)
node("tex_capital", "Austin", "Capital", "Texas", 30.27, -97.74, tag="Live music, cold brisket and a very long memory.")
node("tex_dallas", "Dallas", "City", "Texas", 32.78, -96.6, tag="Big hair, bigger highways.")
node("tex_sa", "San Antonio", "City", "Texas", 29.42, -98.49, tag="Remembers. Everything.")
node("tex_elpaso", "El Paso", "City", "Texas", 31.76, -106.49, tag="Far West Texas, closer to everyone else.")
node("tex_permian", "Permian Refinery", "Fuel", "Texas", 31.95, -102.1, tag="Pumps double. Glows on every radar.")
node("tex_corpus", "Corpus Christi Refinery", "Fuel", "Texas", 27.8, -97.4, tag="Gulf breeze, crude smell.")
node("tex_factory", "Fort Worth Works", "Factory", "Texas", 32.75, -97.7, tag="Where the West begins, and the bombers too.")
node("tex_ab_abilene", "Abilene Field", "Airbase", "Texas", 32.42, -99.85, tag="Home of the Longhorn's hangar.")
node("tex_ab_lubbock", "Lubbock Field", "Airbase", "Texas", 33.58, -101.86, tag="Wind, cotton and afterburners.")
node("tex_ab_delrio", "Del Rio Field", "Airbase", "Texas", 29.36, -100.78, tag="Hot as a griddle, twice as busy.")
node("tex_radar_bigbend", "Big Bend Radar", "Radar", "Texas", 29.6, -103.4, tag="Darkest skies in Texas, brightest screen.")
node("tex_radar_wf", "Wichita Falls Radar", "Radar", "Texas", 33.91, -98.49, tag="Keeps an eye on Oklahoma. Always.")
node("tex_ad_galveston", "Galveston Battery", "AirDefense", "Texas", 29.6, -95.4, tag="Guards the refinery from the beach.")
node("tex_ad_waco", "Waco Battery", "AirDefense", "Texas", 31.55, -97.15, tag="Right in the middle of I-35.")
node("tex_jct_stockton", "Fort Stockton Junction", "Junction", "Texas", 30.89, -102.88, tag="Pipelines from every direction.")
node("tex_jct_amarillo", "Amarillo Junction", "Junction", "Texas", 35.22, -101.83, tag="Steak, cattle and Route 66.")
node("tex_jewel", "The Gulf Refinery", "CrownJewel", "Texas", 29.74, -95.01, tag="Half of Texas's fuel, all of its pride.")

# Grain Belt Union
node("grb_capital", "Lincoln", "Capital", "GrainBelt", 40.81, -96.7, tag="Capital of the breadbasket.")
node("grb_okc", "Oklahoma City", "City", "GrainBelt", 35.47, -97.52, tag="Cattle, wind and a skyline that tries.")
node("grb_fargo", "Fargo", "City", "GrainBelt", 46.88, -96.79, tag="Cold, kind and well prepared.")
node("grb_siouxfalls", "Sioux Falls", "City", "GrainBelt", 43.55, -96.73, tag="Falls, banks and good manners.")
node("grb_cushing", "Cushing Hub", "Fuel", "GrainBelt", 35.98, -96.77, tag="Pipeline crossroads of the plains.")
node("grb_williston", "Williston Oil", "Fuel", "GrainBelt", 48.15, -103.62, tag="Flares on the horizon, all night long.")
node("grb_factory", "Air Capital Works", "Factory", "GrainBelt", 37.69, -97.34, tag="Wichita has been building planes forever.")
node("grb_ab_minot", "Minot Field", "Airbase", "GrainBelt", 48.23, -101.3, tag="Why not Minot? Freezin's the reason.")
node("grb_ab_rapid", "Rapid City Field", "Airbase", "GrainBelt", 44.08, -103.23, tag="Gateway to the Black Hills.")
node("grb_ab_enid", "Enid Field", "Airbase", "GrainBelt", 36.4, -97.88, tag="Wheat to the horizon in every direction.")
node("grb_radar_platte", "North Platte Radar", "Radar", "GrainBelt", 41.12, -100.77, tag="Sees you coming from Wyoming.")
node("grb_radar_pierre", "Pierre Radar", "Radar", "GrainBelt", 44.37, -100.35, tag="Pronounced 'peer'. Sees everything.")
node("grb_ad_grandforks", "Grand Forks Silos", "AirDefense", "GrainBelt", 47.92, -97.03, tag="Old silos, new tricks.")
node("grb_ad_greatbend", "Great Bend Silos", "AirDefense", "GrainBelt", 38.36, -98.76, tag="Cheap, cheerful and very loud.")
node("grb_ad_kimball", "Kimball Silos", "AirDefense", "GrainBelt", 41.24, -103.66, tag="The panhandle bites back.")
node("grb_jct_kearney", "Kearney Junction", "Junction", "GrainBelt", 40.7, -99.08, tag="Sandhill cranes and the Union Pacific.")
node("grb_jct_salina", "Salina Junction", "Junction", "GrainBelt", 38.84, -97.61, tag="Where I-70 meets I-135 and nobody's in a hurry.")
node("grb_jct_aberdeen", "Aberdeen Junction", "Junction", "GrainBelt", 45.46, -98.49, tag="Grain elevators as far as the eye can see.")
node("grb_jewel", "World's Largest Ball of Twine", "CrownJewel", "GrainBelt", 39.51, -98.43, tag="Disputed. Defended. Beloved.")

# The Big Empty (neutral)
node("be_boneyard", "The Boneyard", "Boneyard", None, 32.17, -110.85, 2, "Thousands of old planes, some of them still willing.")
node("be_massdriver", "White Sands Mass Driver", "MassDriver", None, 32.9, -106.4, 2, "A half-finished space launcher. Now a very big gun.")
node("be_ts_kingman", "Kingman Truck Stop", "TruckStop", None, 35.19, -114.05, 1, "Route 66 fuel cache. Pie included.")
node("be_ts_flagstaff", "Flagstaff Truck Stop", "TruckStop", None, 35.2, -111.65, 1, "Ponderosa pines and diesel.")
node("be_ts_gallup", "Gallup Truck Stop", "TruckStop", None, 35.53, -108.74, 1, "Neon motel, full tanks.")
node("be_ts_tucumcari", "Tucumcari Truck Stop", "TruckStop", None, 35.17, -103.72, 1, "Tucumcari tonight! Two thousand motel rooms, one fuel cache.")
node("be_ab_twinfalls", "Twin Falls Field", "Airbase", None, 42.56, -114.46, 1, "Abandoned airbase on the Snake River.")
node("be_ab_greatfalls", "Great Falls Field", "Airbase", None, 47.5, -111.3, 1, "Abandoned airbase under the big sky.")
node("be_ab_casper", "Casper Field", "Airbase", None, 42.87, -106.31, 1, "Abandoned airbase. The wind kept it clean.")
node("be_ab_albuquerque", "Albuquerque Field", "Airbase", None, 35.04, -106.61, 1, "Abandoned airbase. Balloon fiesta every October anyway.")
node("be_boise", "Boise", "City", None, 43.62, -116.2, 0, "Potatoes, rivers and nobody's flag.")
node("be_billings", "Billings", "City", None, 45.78, -108.5, 0, "The biggest city in the biggest sky.")
node("be_phoenix", "Phoenix", "City", None, 33.45, -112.07, 0, "A dry heat. An unclaimed city.")
node("be_jct_cheyenne", "Cheyenne Junction", "Junction", None, 41.14, -104.82, 0, "Rails, rodeos and an empty capitol.")
node("lake_flathead", "Flathead Lake", "Lake", None, 47.88, -114.13, tag="Cherry orchards and floatplane docks.")
node("lake_yellowstone", "Yellowstone Lake", "Lake", None, 44.46, -110.37, tag="Geysers optional.")
node("lake_powell", "Lake Powell", "Lake", None, 36.95, -111.3, tag="Red rock canyons full of water.")

# --- Links ---------------------------------------------------------------------
L = [
    # Cascadia
    ("cas_refinery", "cas_seattle", "Pipeline"),
    ("cas_seattle", "cas_capital", "Rail"),
    ("cas_seattle", "cas_jct_yakima", "Highway"),
    ("cas_spokane", "cas_ab_moses", "Highway"),
    ("cas_ab_moses", "cas_jct_yakima", "Highway"),
    ("cas_jewel", "cas_spokane", "Highway"),
    ("cas_factory", "cas_jct_yakima", "Rail"),
    ("cas_jct_yakima", "cas_capital", "Rail"),
    ("cas_capital", "cas_jct_bend", "Highway"),
    ("cas_jct_bend", "cas_ab_klamath", "Highway"),
    ("cas_spokane", "cas_factory", "Rail"),
    # Pacific
    ("pac_sac", "pac_capital", "Rail"),
    ("pac_la", "pac_oil", "Highway"),
    ("pac_oil", "pac_jct_fresno", "Rail"),
    ("pac_jct_fresno", "pac_capital", "Rail"),
    ("pac_sd", "pac_la", "Rail"),
    ("pac_la", "pac_jct_barstow", "Rail"),
    ("pac_jct_barstow", "pac_factory", "Highway"),
    ("pac_factory", "pac_oil", "Highway"),
    ("pac_jct_barstow", "pac_ab_29", "Highway"),
    ("pac_sac", "pac_ab_fairfield", "Highway"),
    ("pac_ab_fairfield", "pac_capital", "Highway"),
    ("pac_sac", "pac_ab_redding", "Highway"),
    ("pac_jewel", "pac_capital", "Highway"),
    # Dreamland
    ("drm_reno", "drm_ab_fallon", "Highway"),
    ("drm_ab_fallon", "drm_jct_hawthorne", "Highway"),
    ("drm_jct_hawthorne", "drm_ab_tonopah", "Highway"),
    ("drm_ab_tonopah", "drm_capital", "Highway"),
    ("drm_oil", "drm_ab_tonopah", "Highway"),
    ("drm_oil", "drm_jct_caliente", "Highway"),
    ("drm_jct_caliente", "drm_capital", "Rail"),
    ("drm_laughlin", "drm_capital", "Highway"),
    ("drm_laughlin", "drm_jewel", "Highway"),
    ("drm_jewel", "drm_capital", "Highway"),
    ("drm_factory", "drm_capital", "Highway"),
    ("drm_factory", "drm_ab_tonopah", "Highway"),
    # Beehive
    ("bee_factory", "bee_capital", "Rail"),
    ("bee_jct_spike", "bee_factory", "Rail"),
    ("bee_ab_wendover", "bee_capital", "Highway"),
    ("bee_provo", "bee_capital", "Rail"),
    ("bee_stgeorge", "bee_provo", "Highway"),
    ("bee_oil", "bee_capital", "Highway"),
    ("bee_oil", "bee_jct_green", "Highway"),
    ("bee_jct_green", "bee_provo", "Rail"),
    ("bee_ab_moab", "bee_jct_green", "Highway"),
    ("bee_jewel", "bee_capital", "Highway"),
    # Front Range
    ("frs_brew_fc", "frs_capital", "Highway"),
    ("frs_brew_steamboat", "frs_jct_glenwood", "Highway"),
    ("frs_brew_steamboat", "frs_brew_fc", "Highway"),
    ("frs_jct_glenwood", "frs_capital", "Rail"),
    ("frs_gj", "frs_jct_glenwood", "Rail"),
    ("frs_brew_durango", "frs_brew_alamosa", "Highway"),
    ("frs_brew_durango", "frs_ab_gunnison", "Highway"),
    ("frs_brew_alamosa", "frs_factory", "Highway"),
    ("frs_factory", "frs_springs", "Rail"),
    ("frs_springs", "frs_capital", "Rail"),
    ("frs_ab_gunnison", "frs_jct_glenwood", "Highway"),
    ("frs_ab_gunnison", "frs_factory", "Highway"),
    ("frs_ab_limon", "frs_capital", "Highway"),
    ("frs_jct_julesburg", "frs_capital", "Rail"),
    ("frs_jewel", "frs_springs", "Highway"),
    # Texas
    ("tex_dallas", "tex_factory", "Highway"),
    ("tex_dallas", "tex_capital", "Rail"),
    ("tex_factory", "tex_capital", "Highway"),
    ("tex_sa", "tex_capital", "Rail"),
    ("tex_corpus", "tex_sa", "Pipeline"),
    ("tex_corpus", "tex_capital", "Highway"),
    ("tex_jewel", "tex_capital", "Pipeline"),
    ("tex_permian", "tex_jct_stockton", "Pipeline"),
    ("tex_jct_stockton", "tex_sa", "Pipeline"),
    ("tex_permian", "tex_ab_abilene", "Highway"),
    ("tex_ab_abilene", "tex_factory", "Highway"),
    ("tex_elpaso", "tex_jct_stockton", "Highway"),
    ("tex_jct_amarillo", "tex_ab_lubbock", "Highway"),
    ("tex_ab_lubbock", "tex_ab_abilene", "Highway"),
    ("tex_ab_delrio", "tex_sa", "Highway"),
    # Grain Belt
    ("grb_fargo", "grb_jct_aberdeen", "Rail"),
    ("grb_jct_aberdeen", "grb_siouxfalls", "Highway"),
    ("grb_siouxfalls", "grb_capital", "Rail"),
    ("grb_williston", "grb_ab_minot", "Rail"),
    ("grb_ab_minot", "grb_jct_aberdeen", "Rail"),
    ("grb_ab_rapid", "grb_jct_aberdeen", "Highway"),
    ("grb_jct_kearney", "grb_capital", "Rail"),
    ("grb_jct_kearney", "grb_jct_salina", "Highway"),
    ("grb_jct_salina", "grb_capital", "Rail"),
    ("grb_factory", "grb_jct_salina", "Rail"),
    ("grb_cushing", "grb_okc", "Highway"),
    ("grb_cushing", "grb_factory", "Pipeline"),
    ("grb_okc", "grb_ab_enid", "Highway"),
    ("grb_ab_enid", "grb_factory", "Highway"),
    ("grb_jewel", "grb_jct_salina", "Highway"),
    ("grb_fargo", "grb_siouxfalls", "Highway"),
    # Between regions (matter once a node is captured)
    ("cas_ab_klamath", "pac_ab_redding", "Highway"),
    ("cas_jct_bend", "be_boise", "Highway"),
    ("cas_spokane", "be_ab_greatfalls", "Highway"),
    ("pac_sac", "drm_reno", "Highway"),
    ("pac_ab_29", "be_phoenix", "Highway"),
    ("drm_laughlin", "be_phoenix", "Highway"),
    ("drm_jct_caliente", "bee_stgeorge", "Highway"),
    ("drm_ab_fallon", "be_ab_twinfalls", "Highway"),
    ("be_boise", "be_ab_twinfalls", "Highway"),
    ("be_ab_twinfalls", "bee_jct_spike", "Highway"),
    ("be_ab_greatfalls", "be_billings", "Highway"),
    ("be_billings", "be_ab_casper", "Highway"),
    ("be_ab_casper", "be_jct_cheyenne", "Highway"),
    ("be_jct_cheyenne", "frs_capital", "Highway"),
    ("be_jct_cheyenne", "grb_jct_kearney", "Rail"),
    ("bee_ab_moab", "frs_gj", "Highway"),
    ("bee_capital", "be_ab_casper", "Highway"),
    ("be_phoenix", "be_boneyard", "Highway"),
    ("be_boneyard", "be_massdriver", "Highway"),
    ("be_massdriver", "be_ab_albuquerque", "Highway"),
    ("be_massdriver", "tex_elpaso", "Highway"),
    ("be_ab_albuquerque", "frs_brew_alamosa", "Highway"),
    ("frs_jct_julesburg", "grb_jct_kearney", "Rail"),
    ("frs_ab_limon", "grb_jct_salina", "Highway"),
    ("tex_jct_amarillo", "grb_okc", "Highway"),
    ("tex_dallas", "grb_okc", "Rail"),
    ("be_billings", "grb_ab_rapid", "Highway"),
]

# --- Projection and relaxation -------------------------------------------------
MIN_SPACING = 27.0

for n in N:
    x, y = proj(n["lat"], n["lon"])
    n["x0"], n["y0"] = x, y
    n["x"], n["y"] = x, y

for _ in range(400):
    moved = False
    for i in range(len(N)):
        for j in range(i + 1, len(N)):
            a, b = N[i], N[j]
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
    for n in N:  # weak spring back to the real location
        n["x"] += (n["x0"] - n["x"]) * 0.02
        n["y"] += (n["y0"] - n["y"]) * 0.02
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


def zone_of(x, y):
    for zid, *_ in ZONES:
        if pip(x, y, zone_polys[zid]):
            return zid
    return None


REGION_ZONES = {}
for zid, name, region, pts in ZONES:
    REGION_ZONES.setdefault(region, []).append(zid)

problems = []
for n in N:
    region = n["owner"] or "BigEmpty"
    z = zone_of(n["x"], n["y"])
    if z is None or (n["kind"] != "Lake" and z not in REGION_ZONES[region]):
        # fall back to the real location's zone
        z0 = zone_of(n["x0"], n["y0"])
        problems.append((n["id"], z, z0))
        n["x"], n["y"] = n["x0"], n["y0"]
        z = z0
    n["zone"] = z

import sys
for p in problems:
    print("// note: node %s relaxed out of its zone (%s), kept real position in %s" % p, file=sys.stderr)


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


ids = {n["id"] for n in N}
for a, b, k in L:
    assert a in ids and b in ids, (a, b)

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
out.append("    nodes: [")
for n in N:
    owner = "Some(%s)" % n["owner"] if n["owner"] else "None"
    out.append("        NodeDef(id: \"%s\", name: \"%s\", kind: %s, owner: %s, zone: \"%s\", pos: %s, objective: %d, tagline: \"%s\")," % (
        n["id"], n["name"], n["kind"], owner, n["zone"], ipt((n["x"], n["y"])), n["objective"], n["tag"].replace('"', '\\"')))
out.append("    ],")
out.append("    links: [")
for a, b, k in L:
    out.append("        LinkDef(a: \"%s\", b: \"%s\", kind: %s)," % (a, b, k))
out.append("    ],")
out.append(")")
print("\n".join(out))
