<p align="center">
  <img src="./logo.png" />
</p>

<h3 align="center">A simple VPN</h3>

# How to use

Start a server

```sh
obirt server
```

Start a client using `obirt.henrybarreto.dev` as server.

```sh
obirt client obirt.henrybarreto.dev
```

## The Client

The client connects to a server, send all IP packages received by the network interface and injecting the package what was routed by the server.

```mermaid
sequenceDiagram
participant interface
participant client
participant server
client ->>+ server: Send Mac address
client ->> client: Generate key pair
client ->> server: Send peer Public Key
server -->> client: Send server Public Key
server ->> server: Store the client information in the server
server -->> client: Send IP and Mask to bind
client ->>+ interface: Create network interface

loop Keep alive
    client ->> server: Send keep alive package
end

loop Every IP package
    par From Client
    interface ->> client: Receive the IP package
    client ->> client: Encrypt the package
    client ->> server: Send the IP package to server
    server ->> server: Look for the package's destination and sent it
    end
    par From Server
    server ->> client: Send the IP package to client
    client ->> client: Decrypt the package
    client ->> interface: Send the IP package
    end
end

client ->> server: Close
server ->>- client: Close
client ->> interface: Close network interface
interface ->>- client: Confrim close on network interface
```

## The Server

The server receives all IP packages from the peers connected, and redirect it to the destination registed on it.

<p align="center">. . .</p>
