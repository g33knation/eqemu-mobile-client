import socket
import struct
import sys

# Constants
LOGIN_IP = "192.168.1.21"
LOGIN_PORT = 5998
OP_SESSION_REQUEST = 0x01
PROTOCOL_VERSION = 4
MAX_PACKET_SIZE = 512

def test_handshake():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(2.0)
    
    # Construct Packet:
    # struct ReliableStreamConnect {
    #   u8 zero;
    #   u8 opcode;
    #   u32 protocol_version; (BE)
    #   u32 connect_code; (BE)
    #   u32 max_packet_size; (BE)
    # }
    # Total 14 bytes.
    # Format: !BBIII (!=Network/BE, B=u8, I=u32)
    
    connect_code = 12345678
    
    packet = struct.pack("!BBIII", 
                         0, 
                         OP_SESSION_REQUEST, 
                         PROTOCOL_VERSION, 
                         connect_code, 
                         MAX_PACKET_SIZE)
    
    print(f"Sending SessionRequest to {LOGIN_IP}:{LOGIN_PORT}")
    print(f"Hex: {packet.hex()}")
    
    try:
        sock.sendto(packet, (LOGIN_IP, LOGIN_PORT))
        
        data, addr = sock.recvfrom(1024)
        print(f"Received {len(data)} bytes from {addr}")
        print(f"Hex: {data.hex()}")
        
        # Parse reply
        # struct ReliableStreamConnectReply {
        #   u8 zero; u8 opcode; u32 connect_code; ...
        # }
        if len(data) >= 2:
            opcode = data[1]
            if opcode == 0x02: # OP_SessionResponse
                print("SUCCESS: Session Response Received!")
            else:
                print(f"Received unexpected opcode: {opcode}")
        
    except socket.timeout:
        print("TIMED OUT - No response")
    except Exception as e:
        print(f"Error: {e}")
    finally:
        sock.close()

if __name__ == "__main__":
    test_handshake()
