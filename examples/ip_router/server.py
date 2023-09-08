import socket

def main():
    # Create a socket object
    # server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    # 
    # # Bind the socket to a specific IP address and port
    # server_address = ('localhost', 8000)  # Change this to your desired IP and port
    # server_socket.bind(server_address)
    # 
    # # Listen for incoming connections
    # server_socket.listen(0)
    # print("Server is listening for incoming connections...")
    
    # while True:
    #     # Accept a client connection
    #     client_socket, client_address = server_socket.accept()
    #     print(f"Connection established with {client_address}")
    #     
    #     # Send a message to the client
    #     message = "Hello, client! This is the server."
    #     client_socket.send(message.encode('utf-8'))
    #     
    #     # Close the client socket
    #     client_socket.close()
    # Create a socket object
     server_socket = socket.socket(socket.AF_INET, socket.SOCK_RAW, socket.IPPROTO_TCP)
     
     # Bind the socket to a specific IP address and port
     server_address = ('127.0.0.1', 8081)  # Change this to your desired IP and port
     server_socket.bind(server_address)
     
     # Listen for incoming connections
     # server_socket.listen(5)
     print("Server is listening for incoming connections...")
     
     while True:
         print("Receive data...")
         bytes, address = server_socket.recvfrom(4096)
         if address[0] != '172.17.0.1':
             print("Not from client")
             continue

         if address[1] != 8080:
             print("Not from 8080")
             continue

         # get the IP address of the client

         client = bytes[12:16]
         # to string
         print("Client IP: ", '.'.join(map(str, client)))
         # port
         port = bytes[20:22]
         print("Client Port: ", int.from_bytes(port, byteorder='big'))

         # tcp flags
         flags = bytes[33]
         print("Flags: ", flags)

         # tcp payload
         payload = bytes[40:]
         print("Payload: ", payload)

         # address = client + port
         address = ('.'.join(map(str, client)), int.from_bytes(port, byteorder='big'))

         # Send a message to the client
         message = "Hello, client! This is the server."
         # client_socket.send(message.encode('utf-8'))
         server_socket.sendto(message.encode('utf-8'), address)

         
         # Close the client socket
         # client_socket.close()
         print("Client connection closed.")
         server_socket.close()
if __name__ == "__main__":
    main()
