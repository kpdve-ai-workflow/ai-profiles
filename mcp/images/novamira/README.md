# Novamira

Wspólny katalog projektu: `.ai-profiles/mcp/novamira/`.
ZIP: oficjalny release `use-novamira/novamira` 1.12.12, pełna paczka z vendor/.
SHA-256: `ad4dac087e9cad073b2206cb75b44bf5224d57880452f1937a038068920caf78`.

ZIP jest częścią repozytorium i instalator profilu kopiuje go do bundle’a;
instalacja profilu nie wymaga pobierania go z sieci ani nie dotyka WordPressa.
Sam uruchom jedną z komend w projekcie:

```sh
.ai-profiles/mcp/novamira/install-plugin --wordpress /sciezka/do/wordpressa
# albo dla istniejącego, uruchomionego DDEV (skrypt go nie uruchamia):
.ai-profiles/mcp/novamira/install-plugin --ddev
```

Wymagane WP-CLI (w wariancie DDEV wewnątrz DDEV), środowisko WordPressa `local`
lub `development` i brak istniejącej wtyczki Novamira. Skrypt nie aktualizuje ani
nie nadpisuje istniejącej instalacji. Weryfikuje ZIP i używa WordPressowego
instalatora, który respektuje WP_PLUGIN_DIR. Nie aktywuje wtyczki sieciowo ani lokalnie.

W panelu **docelowej witryny** aktywuj plugin i AI Abilities. Wygeneruj
**hasło aplikacyjne WordPressa** (nie hasło wp-admin). Użytkownik samodzielnie
tworzy `<PROJECT>/.env.novamira`, nadaje mu `chmod 600 .env.novamira` i lokalnie
wprowadza dane. Plik jest wymagany do uruchomienia MCP, ale nie do instalacji.
Przykładowa zawartość (fikcyjne dane):

```dotenv
WP_API_URL=https://wordpress.example.test/wp-json/mcp/novamira
WP_API_USERNAME=przykladowy-login
WP_API_PASSWORD="xxxx xxxx xxxx xxxx xxxx xxxx"
```

Wymagane pola to URL endpointu, login konta WordPress i jego hasło aplikacyjne.
Nie commituj pliku ani nie udostępniaj przez HTTP; nie wpisuj sekretów w
komendach, JSON klienta ani czacie. Instalator nie tworzy ani nie kopiuje pliku.

Zbuduj obraz (bez sekretów, ZIP-a, prywatnych plików i projektu w kontekście):

```sh
docker build -t ai-profiles-novamira:0.1.0 .ai-profiles/mcp/novamira
```

Potem restart klienta/MCP. Adapter Rust czyta env, a launcher uruchamia most
0.4.0 wewnątrz Docker Compose przez stdio, bez hostowego Node/npm/npx. Nie ma
automatycznego buildu ani pull. Linux host networking udostępnia istniejącą
sieć hosta; nie konfigurujemy DDEV, VPN ani DNS. Projekt i sekrety nie są
montowane do kontenera. Dane połączenia są przekazywane przez environment,
nie argumenty. Administrator demona Docker może je odczytać.

Dla lokalnego HTTPS umieść wyłącznie publiczny CA w `.private/ca.crt` w tym
katalogu (bez symlinków, nigdy klucz prywatny). Launcher montuje ten jeden plik
RO. Nie wyłączaj TLS. Pusty env i błędy inicjalizacji są zgłaszane przez istniejącą
obsługę MCP; HTTP 401 nie jest mylone z błędem TLS, DNS czy brakiem narzędzi.

Poświadczenia są w katalogu głównym projektu, niezależnie od katalogu roboczego
terminala i położenia launchera. Ścieżki zasobów Novamiry są stałe; lokalne
command/cwd/env_file nie są obsługiwane. Nie ma migracji starych lokalizacji.
Wspólny bundle jest przygotowywany jednokrotnie, a różniąca się zawartość receptur
lub ZIP-a blokuje operację zamiast nadpisania. Kolejny klient używa tego samego
bundle bez nadpisywania sekretów. Uninstall zachowuje bundle i ZIP oraz nie usuwa
pliku poświadczeń użytkownika, pluginu WordPressa ani obrazu Docker.
