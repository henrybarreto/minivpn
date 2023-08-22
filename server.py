import socket

# Server IP address and port
server_ip = '127.0.0.1'   # Listen on all available network interfaces
server_port = 8000     # Port to listen on

# Create a UDP socket
server_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

# Bind the socket to the IP address and port
server_socket.bind((server_ip, server_port))

print("UDP server is listening on", server_ip, "port", server_port)

while True:
    # Receive data and address from the client
    data, client_address = server_socket.recvfrom(1024)
    
    print("Received data from", client_address, ":", data.decode())
    
    # Send a response back to the client
    response = "Hello, client!, I am UDP server."
    server_socket.sendto(response.encode(), client_address)
