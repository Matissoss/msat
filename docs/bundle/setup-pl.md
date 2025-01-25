# Konfiguracja msat

1. Pobierz lub zbuduj **msat** ze źródła,
    a) W przypadku pobierania:
        - Wejdź w sekcję nazwaną: "releases" i pobierz **msat** nazwany z twoim SO (pierwsze 3 litery: lin - Linux, win - Windows) 
        i dopasowaną architekturę CPU (x86_64 (64-bitowe x86) or aarch64 (64-bitowy ARM)).
        - Pobierz plik z rozszerzeniem `.tar.gz`
        - Zdekompresuj go używając preferowanego programu dekompresującego (np. 7zip, winrar, Ark, tar, itp.), który wspiera format `.tar.gz`
    b) W przypadku budowanie ze źródła:
        - Instrukcje można znaleźć: [README.md](https://github.com/Matissoss/msat/tree/main/README.md) pod
        sekcją nazwaną: "Building from source"
2. Skonfiguruj **msat** za pomocą pliku w `data/config.toml`
W miejsce "password" wstaw wybrane przez ciebie hasło. **Hasło MUSI/MOŻE MIEĆ TYLKO JEDNO SŁOWO**, inaczej **ZAPYTANIA** nie zadziałają.

***(OPCJONALNE)*** W miejsce "tcp_ip" wstaw poprawne IPv4, preferowalnie publiczne, by pozwolić zewnętrzenemu urządzeniu połączenie się 
z nim. 
Jeśli jeszcze nie chcesz wstawiać IPv4, wstaw w to miejsce: "127.0.0.1" (lokalne IPv4).

3. Teraz, uruchom binarkę/plik wykonywalny nazwany: `admin_dashboard` i `app_server`  w tym samym momencie
    a) Jeśli używając powłoki bash-compatible/terminal: `./admin_dashboard & ./app_server`,
    b) Jeśli na Windows/Linux z Środowiskiem Graficznym (nie w trybie tekstowym) otwórz je jedno po drugim.
4. Połącz się z panelem administratorskim **msat** poprzez wstawienie w przeglądrkę następującego URL: "localhost:8000" 
(jeśli ustawiłeś ip_addr na "127.0.0.1")
5. Sprawdź czy panel administratorski działa poprzez egzekwowanie kilku komend i wstawieniu hasła w sekcję z wejściem. Powinieneś dostać informację zwrotną jeśli działa lub nie działa. 

(Uwaga): obecnie `./app_server` nie ma żadnego użytku
