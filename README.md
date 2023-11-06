# Rust SQL Server

Why?  Because why not.  I'm just poking around with rust, seeing what I can do.  No intention of this project going
anywhere.

## Architecture

The server is split into (at least) two parts - the main server part only listens to a unix socket for commands.  A
second binary is a multiplexer/router that listens on two separate ports:

Port 4201 -> query port: send queries to this port
Port 4202 -> command port: used by other multiplexers on other servers to communicate

Replication is done via port 4202 using REPL commands followed by data streams from the primary.

## Project Goals

### Server

[x] SQL parsing (done with a library)
[ ] CREATE TABLE support
[ ] SELECT support
[ ] INSERT support
[ ] DELETE support
[ ] Transactions (BEGIN/COMMIT/ROLLBACK)

### Multiplexer

[ ] SQL parsing (same library that server uses)
[ ] forward supported queries to local unix socket
[ ] discover (auto or manual) other multiplexers
[ ] load-balancing between multiplexers
[ ] multiple write nodes (gotta drop ACID for this I think)

