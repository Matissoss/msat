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
2. Stwórz ścieżkę/katalog/folder nazwany: "data" w głównej ścieżce/katalogu/folderze
3. Stwórz plik `config.toml` w ścieżce "data".
4. Dodaj następującą zawartość do `config.toml`:
    ```toml 
    password="WSTAW_HASŁO"
    ip_addr ="WSTAW_IPv4"
    ```
W miejsce "WSTAW_HASŁO" wstaw wybrane przez ciebie hasło. **Hasło MUSI/MOŻE MIEĆ TYLKO JEDNO SŁOWO**, inaczej **ZAPYTANIA** nie zadziałają.

***(OPCJONALNE)*** W miejsce "WSTAW_IPv4" wstaw poprawne IPv4, preferowalnie publiczne, by pozwolić zewnętrzenemu urządzeniu połączenie się 
z nim. 
Jeśli jeszcze nie chcesz wstawiać IPv4, wstaw w to miejsce: "127.0.0.1" (lokalne IPv4, tylko dostępne przez LAN).

5. Teraz, uruchom binarkę/plik wykonywalny nazwany: `admin_dashboard` i `app_server`  w tym samym momencie
    a) Jeśli używając powłoki bash-compatible/terminal: `./admin_dashboard & ./app_server`,
    b) Jeśli na Windows/Linux z Środowiskiem Graficznym (nie w trybie tekstowym) otwórz je jedno po drugim.
6. Połącz się z panelem administratorskim **msat** poprzez wstawienie w przeglądrkę następującego URL: "localhost:8000" 
(jeśli ustawiłeś ip_addr na "127.0.0.1")
7. Sprawdź czy panel administratorski działa poprzez egzekwowanie kilku komend i wstawieniu hasła w sekcję z wejściem. Powinieneś dostać informację zwrotną jeśli działa lub nie działa. 
8. Szczęśliwego Używania [^1]

[^1]: Ta część będzie rozszerzona, kiedy klient **msat**, **msatc** zostanie skończony.
