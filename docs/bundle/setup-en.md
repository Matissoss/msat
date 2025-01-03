# Setup msat

1. Download or build **msat** from source,
    a) In case of downloading:
        - Go to releases section and download **msat** named with your OS (first 3 letters: lin - Linux, win - Windows) 
        and corresponding CPU architecture (x86_64 (64-bit x86) or aarch64 (64-bit ARM)).
        - Download file ending with `.tar.gz`
        - Uncompress it with your favourite uncompressing software (like 7zip, winrar, Ark, tar, etc.) that supports `.tar.gz` format
    b) In case of building from source:
        - Instructions can be found in: [README.md](https://github.com/Matissoss/msat/tree/main/README.md) under 
        section named: "Building from source"
2. Create directory named: "data" in main directory
3. Create `config.toml` file in "data" directory.
4. Add following into `config.toml`:
    ```toml 
    password="INSERT_PASSWORD"
    ip_addr ="INSERT_IPv4"
    ```
In place of "INSERT_PASSWORD" insert your password of choice. **Password MUST BE ONE-WORD**, otherwise **REQUESTS** won't work.

***(OPTIONAL)*** In place of "INSERT_IPv4" insert correct IPv4, preferably public one, so any device can connect to it. 
If you don't want to insert IPv4 just yet, insert in this place: "127.0.0.1" (local IPv4, only accesible through LAN)..

5. Now, you want to start binary/executable files named: `admin_dashboard` and `app_server` at once
    a) If using bash-compatible shell/terminal: `./admin_dashboard & ./app_server`,
    b) If on Windows/Linux with Desktop Enviroment (not in text mode) open them one after another.
6. Connect to admin dashboard inserting this into your browser URL Search: "localhost:8000" (if you did set ip_addr to "127.0.0.1")
7. Test if admin dashboard works by executing some commands and insert your password into Input Section. You should get feedback, if it 
works/doesn't work 
8. Enjoy [^1]

[^1]: This will be expanded, when **msat** client, **msatc** will be finished.
