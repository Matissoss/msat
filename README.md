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
**[app server](app_server)** and **[admin dashboard](admin_dashboard)**

**msatc** is client for **msat** made exclusively for mobile platforms (TODO).

# Features

- 🚀 Multithreaded and asynchronous,
- 🦀 Made in Rust,
- 🔑 Self-hostable,
- 📖 User-friendly documentation,
- 💤 Customizable admin-dashboard
- 🆓 FOSS (Free and open-source)

# Download Pre-compiled Binaries

It's recommended way to download **msat**. Go to releases section and find release you want to use (preferably with `stable` in its name). 
Then download compressed file for your operating system and CPU architecture (for non-technical users: your machine in most cases ***PROBABLY*** 
uses 64-bit x86 - x86_64).

# Dependencies 

**msat** requires following dependencies to function:
- **curl**

# Building from Source 

## msat 
**msat** requires following dependencies to be built from source:
- **rustc** (preferably cargo) for Rust edition 2021 (may work with earlier editions, but it's NOT officially supported),
- **tar** for compressing build,
- **git** (***optional***) for downloading source code,
- **bash** (***for method 1***) or just shell/terminal compatible with commands: *rm*, *mkdir*, *cp*, *mv*.
   
### Building guide
- 1. Clone repo with **git** (`git clone https://github.com/Matissoss/msat.git`) or download source code from releases section,
- 2. Go into `ci` directory,
- 3. Use command: `./build.sh` and follow instructions
## msatc 
> **msatc**'s development is not started yet

# Versioning

Versioning can be found [here](https://github.com/Matissoss/Matissoss/tree/main/VERSIONING.md)

# License

**msat** and **msatc** and all components included in this repo (except logos in `/promo/` directory) are 

licensed under [**X11 (MIT) License**](LICENSE)

# Credits

**msat** was made using Rust, serde (toml), tokio-rs and SQLite bindings for Rust (rusqlite), 
without these **msat** wouldn't exist.

**msat** uses `https://api.ipify.org/` to get device's public IP.

**msat** and **msatc** were made by **Mateus Dev** ([Profile](https://github.com/Matissoss))
