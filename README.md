# Auction Sniper

Educational Rust not-OOP implemenation of Auction Sniper from "Growing Object-Oriented Software, Guided By Tests" book


More about it in [Data-oriented, clean&hexagonal architecture software in Rust – through an example project](https://dpc.pw/data-oriented-cleanandhexagonal-architecture-software-in-rust-through-an-example)
blog post.

Features:

* Services (main application logical threads/actors) with graceful shutdown on demand or error
* Simple Event-Log-based communication.
* Ports from the Hexagonal Architecture with support for cross-port database transactions
