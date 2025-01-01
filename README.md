<div align=center>
    <img src="promo/logo.svg" width=25%>
    <h1>msat</h1>
    <p>mateus's school administration tool</p>
</div>

---

# ‼️ ATTENTION ‼️

**msat** and **msatc** are in early development and are ***NOT*** deplyoment-ready,
project is approximately **~60%** done before stable version 1.0.

# About

**msat**/**msatc** are FOSS (free and open-source) school administration tools.
**msat** is application-server for **msatc** consisting of: 
**[server](server)** and **[admin dashboard](http_server)**

**msatc** is client for **msat** made exclusively for mobile platforms (TODO).

# Features

- 🚀 Multithreaded and asynchronous,
- 🦀 Made in Rust,
- 🔑 Self-hostable,
- 📖 User-friendly documentation,
- 💤 Customizable admin-dashboard
- 🆓 FOSS (Free and open-source)

# Setup

TODO ([SETUP](setup.md))

# Download Pre-compiled Binaries

It's recommended way to download **msat**. Go to releases section and find release you want to use (preferably with `stable` in its name). 
Then download compressed file for your operating system and CPU architecture (for non-technical users: your machine in most cases ***PROBABLY*** 
uses 64-bit x86 - x86_64).

# Building from Source 

## msat 
**msat** requires following dependencies to be built from source:
- **rustc** (preferably cargo) for Rust edition 2021 (may work with earlier editions, but it's NOT officially supported),
- **tar** for compressing build,
- **git** (***optional***) for downloading source code,
- **bash** (***for method 1***) or just shell/terminal compatible with commands: *rm*, *mkdir*, *cp*, *mv*.
    
1. Clone repo with **git** (`git clone https://github.com/Matissoss/msat.git`) or download source code from releases section,
### Now, there are two ways to build **msat** from scratch:
1. Building using official **build.sh** script:
        - 1. use command: `sh build.sh`
2. Building using **cargo**/**rustc**:
     This is ***NOT*** recommended way to build **msat**, because it requires more effort that is automated with **build.sh** script, but if you want:
   1. Compile *http_server*/*server directory* using `cargo build --release`,
   2. Add directory where you want your server to be stored,
   3. Clone directory named **web** in *http_server* into directory you made,
   4. ***(OPTIONAL)*** Compress build directory using `tar`, `winrar` or your favourite compression software.
## msatc 
> **msatc**'s development is not started yet

# License

**msat** and **msatc** are distributed under either of these two Licenses:
- MPL 2.0 (Mozilla Public License 2.0) ([MPL](LICENSE-MPL.md)),
- Apache License 2.0 ([Apache](LICENSE-APACHE.md))

# Credits

**msat** was made using Rust, serde (toml), tokio-rs and SQLite bindings for Rust (rusqlite), 
without these **msat** wouldn't exist.

**msat** and **msatc** were made by **Mateus Dev** ([Profile](https://github.com/Matissoss))
