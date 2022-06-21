# DHT Exporer - WIP

Implementing a Kademlia DHT Node in Rust for fun

# References

[Rust Cheat Sheet](https://cheats.rs/)

[BitTorrent Enhancement Proposals](https://www.bittorrent.org/beps/bep_0000.html)

[Kademlia whitepaper](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)

[Hobbyist golang implementation](https://github.com/mh-cbon/dht)

[Production golang implementation](https://github.com/anacrolix/dht)

[DHT Tech Talk](https://engineering.bittorrent.com/2013/01/22/bittorrent-tech-talks-dht/)


## License
[MIT](https://choosealicense.com/licenses/mit/)

### Todo
KRPC:

`Ping`
`Store`
`Find_Node(id)` -> returns k closest dht nodes
`Find_Value(id)` -> returns `Store`d value if avaiable, else k closest dht nodes

15 minute refresh

BEPs 20, 42, 32, 33, 44, 51

boostrap|closest_stores|closest_peers|ping|announce_peer|get_peers|find_node|get|put|genkey|sign