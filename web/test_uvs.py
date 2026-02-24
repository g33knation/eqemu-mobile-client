import json

with open("/home/tommy/Desktop/MobileClient/bam_gltf.json", "r") as f:
    gltf = json.load(f)

# Find first mesh's first primitive's TEXCOORD_0 accessor
mesh = gltf["meshes"][0]
prim = mesh["primitives"][0]
if "TEXCOORD_0" in prim["attributes"]:
    accessor_idx = prim["attributes"]["TEXCOORD_0"]
    accessor = gltf["accessors"][accessor_idx]
    
    print(f"Accessor min: {accessor.get('min')}")
    print(f"Accessor max: {accessor.get('max')}")
    print(f"Count: {accessor['count']}")
else:
    print("No TEXCOORD_0")
