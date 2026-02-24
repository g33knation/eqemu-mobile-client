
import json
import struct
import sys
import os

def parse_glb(file_path):
    with open(file_path, 'rb') as f:
        magic = f.read(4)
        if magic != b'glTF':
            print("Not a glTF file")
            return

        version = struct.unpack('<I', f.read(4))[0]
        length = struct.unpack('<I', f.read(4))[0]

        print(f"GLB Version: {version}, Length: {length}")

        while f.tell() < length:
            chunk_length = struct.unpack('<I', f.read(4))[0]
            chunk_type = f.read(4)

            if chunk_type == b'JSON':
                json_data = f.read(chunk_length)
                data = json.loads(json_data)
                
                print("\n--- Animations ---")
                if 'animations' in data:
                    for i, anim in enumerate(data['animations']):
                        print(f"Index {i}: {anim.get('name', 'Unnamed')}")
                else:
                    print("No animations found.")

                print("\n--- Nodes (Top 5) ---")
                if 'nodes' in data:
                    for i, node in enumerate(data['nodes'][:5]):
                         print(f"Node {i}: {node.get('name', 'Unnamed')} - Translation: {node.get('translation', 'None')} - Scale: {node.get('scale', 'None')}")
                
            elif chunk_type == b'BIN\x00':
                # Skip binary data
                f.seek(chunk_length, 1)
            else:
                f.seek(chunk_length, 1)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python inspect_glb.py <file.glb>")
    else:
        parse_glb(sys.argv[1])
