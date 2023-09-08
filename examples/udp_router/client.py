import socket

# Server IP address and port
server_ip = '10.0.0.100'  # Change this to the actual server IP
server_port = 8080      # Change this to the actual server port

# Message to send
message = "Hello, server!, I am UDP client."

# Create a UDP socket raw socket
client_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

# Send the message to the server
client_socket.sendto(message.encode(), (server_ip, server_port))

# Receive response from the server
response, server_address = client_socket.recvfrom(1024)
print("Server response:", response.decode())

# Close the socket
client_socket.close()
