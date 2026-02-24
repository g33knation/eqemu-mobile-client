import asyncio
import websockets
import json
import sys

async def test_combat():
    uri = "ws://localhost:3030/ws"
    async with websockets.connect(uri) as websocket:
        print("Connected to WebSocket")
        
        target_id = None
        # Wait for initial state with spawns
        while True:
            msg = await websocket.recv()
            data = json.loads(msg)
            if "spawns" in data and len(data["spawns"]) > 0:
                print(f"Received Zone State with {len(data['spawns'])} spawns")
                # Pick first spawn that is not us (if we knew our ID, but safe to pick first usually)
                # spawns is dict {id: spawn}
                for sid, spawn in data["spawns"].items():
                    if spawn.get("name") != "Juggs": # Adjust if needed
                         target_id = int(sid)
                         print(f"Selected Target: {spawn.get('name')} ({target_id})")
                         break
                if target_id:
                     break
            elif "zone_id" in data:
                 print("Received Zone Info, waiting for spawns...")

        if not target_id:
            print("No target found!")
            return

        # Send Target
        print(f"Sending Target Command: {target_id}")
        await websocket.send(json.dumps({"type": "target", "id": target_id}))
        await asyncio.sleep(0.5)
        
        # Send Attack
        print("Sending Attack Command...")
        await websocket.send(json.dumps({"type": "attack"}))
        
        # Wait for Damage (timeout 10s)
        print("Waiting for Damage...")
        try:
            while True:
                msg = await asyncio.wait_for(websocket.recv(), timeout=10.0)
                data = json.loads(msg)
                if data.get("type") == "damage":
                    print(f"SUCCESS: Received Damage! {data}")
                    return
                else:
                    print(f"Ignored: {data.keys()}")
        except asyncio.TimeoutError:
            print("FAILURE: Timed out waiting for damage.")
            sys.exit(1)

if __name__ == "__main__":
    asyncio.run(test_combat())
