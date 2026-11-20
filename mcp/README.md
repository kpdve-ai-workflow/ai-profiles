# MCP

Dokumentacja wspólnej konfiguracji MCP ai-profiles: rejestru, wyboru przez profile,
lokalnych nadpisań, adaptera stdio i launcherów. Profile instalują wyłącznie
serwery jawnie wybrane z rejestru; obecność definicji lub launchera sama nie
włącza serwera.

## Rejestr i wybór serwerów

Każdy `mcp/servers/<nazwa>.json` jest jedyną centralną definicją danego serwera.
Profil wskazuje nazwy w `profiles/<profil>/mcp.json`, np.:

```json
{"servers": ["figma", "novamira"]}
```

Lista ma unikalne nazwy odpowiadające plikom rejestru; definicje inline nie są
dozwolone. Profil bez MCP używa `{"servers": []}`. Nieznane nazwy, duplikaty,
symlinki i nieprawidłowe definicje są błędami. `mcp/example.json` jest kontrolnym
zestawieniem sumy definicji wybranych przez profile, nie źródłem instalacji.
Po zmianie definicji lub wyboru zaktualizuj zestawienie i uruchom `make validate`.
Walidacja całego repozytorium sprawdza także niewybrane definicje.

Stan projektu zachowuje rozwinięte `mcp_base`, a nie odwołania do repozytorium,
więc adapter nie potrzebuje dostępu do źródeł ai-profiles. Zmiana definicji
wymaga bumpu wyłącznie profili, które ją wybierają.

`install`, `update` i `switch` renderują jeden efektywny zestaw (rejestr plus
`.ai-profiles/<klient>/local.json`) do wybranego OpenCode (`opencode.json`), Codex
(`.codex/config.toml`, zachowywany przez `toml_edit`) i Claude Code (`.mcp.json`).
CLI klientów nie są wymagane do przygotowania konfiguracji. Wpisy są śledzone
oddzielnie w stanie instalacji; kolizje i ręczne zmiany zarządzanych wpisów
blokują aktualizację. Klienci nadal kontrolują zaufanie projektu, zatwierdzanie
MCP i OAuth. Status potwierdza przygotowaną konfigurację, nie runtime connection.

## Lokalne ustawienia projektu

`.ai-profiles/<klient>/local.json` jest plikiem użytkownika/projektu. Menedżer go
nie tworzy, nie zastępuje ani nie usuwa; `uninstall` również go zachowuje.
Przykład do ręcznego skopiowania znajduje się w `mcp/local.example.json`.
Plik nie jest odczytywany bezpośrednio przez OpenCode, tylko przez menedżer
i adapter. Przykładowa struktura:

```json
{
  "mcp": {
    "postgres": {
      "env_file": ".env.stage",
      "environment": {"DATABASE_URI": "{env:STAGE_DATABASE_URL}"}
    }
  }
}
```

`env_file` jest opcjonalną ścieżką względną od katalogu głównego projektu, bez
`..` i symlinków, dostępna tylko dla lokalnych MCP. Użytkownik tworzy plik sam;
install/update/switch nie wymagają jego obecności. Przechowuj pliki sekretów poza Gitem, dodając
je do projektowego `.gitignore`; konfiguruj sekrety przez referencje środowiskowe,
nie wpisując ich w JSON. Kolejność wartości to środowisko odziedziczone, wskazany
plik env, a następnie mapowanie `environment`. Parser pliku env obsługuje
pojedyncze `KEY=value`, cudzysłowy, komentarze i `export`; nie wykonuje powłoki,
nie interpoluje `$VAR`/`${VAR}` i nie obsługuje wartości wieloliniowych.
Brak wymaganego pliku lub zmiennej zatrzymuje start MCP.

Poświadczenia tworzy wyłącznie użytkownik w katalogu projektu. Dla PostgreSQL
można wskazać `<PROJECT>/.env.postgres` przez
lokalne `env_file` (opcjonalny sposób dostarczenia `DATABASE_URI`). Samodzielnie
utwórz plik lokalnym edytorem, ustaw `chmod 600 <nazwa>`, dodaj do `.gitignore`
i nie udostępniaj przez HTTP. Przykładowa zawartość PostgreSQL (fikcyjna):

```dotenv
DATABASE_URI=postgresql://example_user:example_password@db.example.test:5432/example_db
```

PostgreSQL bez pliku może otrzymać URI z odziedziczonego środowiska. Figma jest
zdalnym OAuth MCP i nie wymaga pliku poświadczeń w projekcie. Playwright używa
opcjonalnego `base-auth.json` i opcjonalnego `playwright-auth.json` (cookies/storage
state); Novamira wymaga `.env.novamira` do startu. Szczegóły i formaty są w
[instrukcji Playwright](images/playwright/README.md#http-basic-auth) i
[instrukcji Novamira](../profiles/wordpress/README.md#5-zapisz-lokalne-połączenie).

Obiekty ustawień scalają się po kluczach, tablice (np. `command`) są zastępowane
w całości. Można nadpisać m.in. `command`, `environment`, `cwd`, `enabled`/
`disabled`, `timeout`, a dla zdalnego MCP także `url`, `headers` i `oauth`.
Nie można zmienić transportu ani nadpisać serwera niezarządzanego przez profil;
takie serwery konfiguruj niezależnie w każdym kliencie. Aktualizacja ostrzega o zmianie
upstream pól, które lokalnie nadpisano, bez ujawniania ich wartości. Nadpisania
niepasujące do aktualnego profilu powodują błąd, a nie ciche pominięcie.
Nie edytuj wygenerowanych wpisów MCP; ręczne zmiany blokują update/uninstall.

PostgreSQL MCP ma domyślnie tryb `unrestricted` (read/write w granicach uprawnień
konta). Tryb ustawiaj wyłącznie przez lokalne pole `access_mode`:

```json
{"mcp": {"postgres": {"access_mode": "restricted"}}}
```

Dozwolone wartości to `restricted` i `unrestricted`. Adapter usuwa stary argument
`--access-mode` z komendy i przekazuje dokładnie jeden wynikający z pola;
domyślnie `unrestricted`. Błędne wartości i powielone argumenty kończą się błędem
przed zapisem lub uruchomieniem. Read/write nie oznacza zgody na niezamówione
operacje destrukcyjne ani migracje.

## Adapter

Instalator dostarcza natywne binarium Rust `.ai-profiles/<klient>/ai-profile-mcp` i nadaje
mu prawa wykonania. OpenCode uruchamia je bezpośrednio. Adapter odczytuje
`mcp_base` ze stanu, nakłada lokalne nadpisania, uruchamia komendy bez powłoki,
przekazuje stdio i rozwiązuje środowisko wyłącznie dla procesu MCP. Nie zmienia
środowiska OpenCode. Zdalne MCP uruchamia natywnie każdy klient i nie używają
`env_file`. Osobny `ai-profile-mcp.json` nie jest wymagany; update usuwa jego
wcześniej zarządzaną kopię. Adapter działa bez Python, Cargo i checkoutu;
zależności wybranego serwera muszą być dostępne osobno. Codex i Claude używają
bezwzględnej ścieżki adaptera i argumentu nazwy serwera; sekrety nie są
rozwijane przy instalacji. Po zmianie pliku env lub mapowania zrestartuj MCP;
po zmianie ustawień klienta zrestartuj jego sesję.

Każda lokalna komenda otrzymuje zastrzeżone `AI_PROFILE_PROJECT_DIR`: kanoniczną
ścieżkę projektu odnalezioną z położenia zainstalowanego adaptera w runtime,
nie z `PWD` ani katalogu klienta. Adapter ustawia ją wyłącznie dla podprocesu,
po rozwiązaniu środowiska; odziedziczone wartości, dotenv i lokalne mapowania
nie zastępują jej końcowej wartości. Użytkownik nie musi jej konfigurować.

## Launchery

### Novamira

Novamira ma centralnie `disabled: false`. Każdy klient otrzymuje niezależny bundle:
`.ai-profiles/<klient>/mcp/novamira/`, z Dockerfile, Compose, launcherem, `install-plugin`
i `downloads/novamira.zip`. Instalator kopiuje pełną
paczkę 1.12.12 z oficjalnego GitHub Releases i weryfikuje przypięty SHA-256.
Nie instaluje jej w WordPressie i nie buduje obrazu Docker podczas instalacji profilu.
Plik `.env.novamira` nie jest dostarczany ani tworzony. Użytkownik sam tworzy go
w katalogu głównym projektu z prawami `0600`; brak pliku nie blokuje lifecycle,
a start MCP wskazuje dokładną nazwę brakującego pliku.
Nie dodajemy reguły `.gitignore` dla Novamiry: użytkownik musi sam zabezpieczyć
plik przed ujawnieniem sekretów przez Git i HTTP. Bundle jest zarządzany osobno
w stanie każdego klienta. Uninstall klienta usuwa jego zarządzane pliki bundle’a;
nie usuwa `.env.novamira` ani niezarządzanych plików prywatnych, takich jak CA.
Stary `.ai-profiles/mcp/novamira/` pozostaje bez zmian i nie jest używany.

Przebieg: instalacja profilu → `install-plugin --wordpress KATALOG` lub `--ddev`
→ ręczna aktywacja pluginu i AI Abilities → hasło aplikacyjne i uzupełnienie env
→ ręczny `docker build -t ai-profiles-novamira:0.1.0 .ai-profiles/<klient>/mcp/novamira`
→ restart klienta/MCP. Skrypt wymaga lokalnego/development WordPressa i WP-CLI;
nie zastępuje istniejącego pluginu ani nie aktywuje go. Nie uruchamia DDEV.
Host nie potrzebuje Node/npm; most 0.4.0 jest wewnątrz obrazu.
Kontener używa Linux host networking, montuje RO pliki hosts/resolv.conf i tylko
opcjonalny publiczny CA `.ai-profiles/<klient>/mcp/novamira/.private/ca.crt` z katalogu bundle.
Poświadczenia są przekazywane przez environment, bez wartości w argumentach.
Puste dane blokują start MCP przed próbą logowania. Nie ma obowiązkowego
local.json ani polecenia aktywacji MCP. Lokalny disabled/timeout jest obsługiwany,
ale command/cwd/env_file Novamiry są stałe. Inne lokalne `env_file` są
ścieżkami względnymi od katalogu projektu.

Osłona Novamiry w adapterze Rust odrzuca zastępczą inicjalizację mostu
`@automattic/mcp-wordpress-remote@0.4.0`, gdy obecne jest
`capabilities.experimental.connectionFailed` (także `{}`). Sam znacznik nie
potwierdza HTTP 401. Osłona odczytuje zachowaną przyczynę przez wewnętrzne
`tools/list` — upstream po nieudanej inicjalizacji obsługuje je lokalnie, bez
ponownego HTTP. Tylko potwierdzony status 401 daje wskazówki o haśle aplikacyjnym;
brak szczegółów, TLS, DNS, timeout i 403 dają komunikat ogólny. Odpowiedź błędu
JSON-RPC trafia do istniejącej obsługi MCP klienta (w OpenCode do panelu MCP/TUI),
bez osobnej wtyczki. Surowe logi i plikowy sink logów mostu są wyłączone, stdout
zawiera tylko MCP. Poprawne odpowiedzi i narzędzia są przekazywane; zero narzędzi
nie oznacza błędnego hasła. Pozostałe serwery nadal działają przez zwykłe exec.

WordPress 3.1.0 dostarcza ten układ bez migracji starych lokalizacji.
Szczegóły: [instrukcja WordPress](../profiles/wordpress/README.md).

### Zarządzane pliki launcherów

Opcjonalne zarządzane pliki runtime umieszczaj w `mcp/launchers/<nazwa>/`.
Instalowane są do `.ai-profiles/<klient>/mcp/<nazwa>/` wyłącznie projektom profili wybierających
serwer i podlegają kontroli hashy, konfliktów oraz usuwaniu jak inne zarządzane
pliki. Nie przechowuj w nich sekretów, cache ani artefaktów budowy. Szczegóły
konkretnej integracji znajdują się w dokumentacji właściwego profilu.

## Playwright i audyty WCAG

Pełny test repozytorium uruchamia `make test-playwright` w trwałej VM Debian 13
amd64 zarządzanej przez Incus (Docker i Node.js 24 LTS wewnątrz gościa).
Etap zainstalowanego adaptera wymaga Node na maszynie wykonującej skrypt, czyli
w VM; produkcyjne użycie MCP opisane poniżej go nie wymaga na hoście.
`network_mode: host` w integracji oznacza sieć VM, nie fizycznego hosta.
Wymagania, cache i eksport raportów: [testowanie](../docs/testing.md).

Profile WordPress i React wybierają serwer `playwright` oraz skill
`wcag-audit`. Pozostałe profile nie otrzymują konfiguracji przeglądarki.
`make install CLIENT=<klient> PROFILE=<profil> PROJECT=<projekt>` dostarcza do projektu kompletny
kontekst budowy i konfigurację Compose w `.ai-profiles/<klient>/mcp/playwright/`. Instalacja
nie wymaga Dockera i nie buduje ani nie pobiera obrazu.

### Przygotowanie w repozytorium docelowym

Z katalogu głównego projektu, mając działający Docker Engine i plugin Compose:

```sh
client=opencode # opencode, claude albo codex — zgodnie z instalacją
docker build --tag ai-profiles-playwright:0.2.0 ".ai-profiles/$client/mcp/playwright"
```
Opcjonalnie przygotuj [plik Basic Auth](images/playwright/README.md#http-basic-auth).
Następnie zrestartuj klienta AI i zatwierdź jego natywne zgody projektu/MCP,
jeśli wymagane. Jak dla pozostałych lokalnych MCP, wspólny
adapter `ai-profile-mcp` odczytuje konfigurację i uruchamia jej komendę — tutaj
natywny `playwright-launcher`, który przygotowuje runtime i wykonuje standardowe
`docker compose run` bez TTY. Nie ma publikacji GHCR, tokenów ani automatycznego
buildu/pull obrazu. Nie uruchamiaj
`compose up -d`: każdy klient ma własny kontener i sesję przeglądarki.

Compose montuje projekt pod jego rzeczywistą bezwzględną ścieżką hosta, tylko
do odczytu, a `.ai-profiles/reports/` pod tą samą ścieżką zapisywalnie. Ścieżkę dostarcza
wspólny adapter przez `AI_PROFILE_PROJECT_DIR`, bez konfiguracji użytkownika.
Launcher automatycznie tworzy brakujące `.ai-profiles/reports/` jako bieżący użytkownik przy
starcie (nie podczas builda); kontener dopasowuje do niego UID/GID.
Nie montuje Docker socketu. Opcjonalne
`HOST_UID` i `HOST_GID` pozwalają jawnie wybrać użytkownika.

Dockerfile deklaruje dokładne wersje `@playwright/mcp` i `axe-core`, bez npm
manifestu i lockfile w repozytorium. Node, npm dependencies, Chromium i biblioteki
systemowe są instalowane wyłącznie wewnątrz obrazu. Skrypt axe jest ładowany przez
`--init-script` również po kolejnych nawigacjach. Nie potrzeba hostowego Node/npm,
cache przeglądarki ani CDN. Przypięte wersje bezpośrednie nie gwarantują identycznych
zależności przechodnich w przyszłych rebuildach.

Po aktualizacji dostarczonej receptury użytkownik jawnie ponawia build w projekcie.
Aktualizacja i uninstall nie zarządzają lokalnymi obrazami ani raportami.
React 4.0.0 i WordPress 3.1.0 umieszczają raporty pod `.ai-profiles/reports/`;
plikami poświadczeń zarządza użytkownik w katalogu głównym projektu, bez migracji
i fallbacku do starych lokalizacji. Sieć pozostaje `host`.
Szczegółowa instrukcja jest [dostarczana razem z konfiguracją](images/playwright/README.md).

### Sieć, certyfikaty i artefakty

Na Linuxie z lokalnym Docker Engine domyślną siecią jest `host`, z dostępem do
hostowego `localhost`. Launcher automatycznie dodaje `compose.host.yaml`:
istniejące `/etc/hosts` i `/etc/resolv.conf` są montowane RO, z
`create_host_path: false`. Nie wymaga to `local.json` ani wyboru sieci DDEV.
Wykorzystywana jest zastana łączność hosta, również już działający VPN;
narzędzie nie konfiguruje VPN, DNS, tras ani firewalla i nie naprawia sieci.
Po zmianie środowiska hosta może być potrzebny restart MCP/klienta.
Nie jest to gwarancja obsługi konkretnego firmowego VPN, Docker Desktop ani
zdalnego Engine.

Opcjonalne nadpisania `environment` umieszczaj w
`.ai-profiles/<klient>/local.json`. `AI_PROFILE_PLAYWRIGHT_NETWORK` nieustawione,
puste lub równe `host` włącza host overlay; każdy jawny inny tryb go nie dodaje.
W bridge hostowy loopback DNS/HTTP nie jest loopbackiem kontenera.
Automatyczne rozszerzenia poprzedzają oryginalne `COMPOSE_FILE`, zachowując
priorytet jawnych plików; już obecny overlay nie jest dublowany ani przenoszony.
Zmieniając sieć wyłącznie własnym Compose, sam zadbaj o zgodność mountów.

Przykład świadomego wyboru innej sieci DDEV (sprawdź jej nazwę u siebie):

```json
{
  "mcp": {
    "playwright": {
      "environment": {
        "AI_PROFILE_PLAYWRIGHT_NETWORK": "ddev_default",
        "AI_PROFILE_PLAYWRIGHT_CA_CERT": ".ddev/ai-profile-ca.crt"
      }
    }
  }
}
```

W tym niestandardowym trybie używaj URL dostępnego w tej sieci, np.
`https://<projekt>.ddev.site`, nie
`localhost` hosta. Dla lokalnego HTTPS
launcher automatycznie wybiera publiczny CA: jawny
`AI_PROFILE_PLAYWRIGHT_CA_CERT`, następnie `CAROOT/rootCA.pem`, następnie wynik
odczytowego `mkcert -CAROOT` i jego `rootCA.pem`, jeśli program jest już na PATH.
Zapytanie ma limit 5 sekund i ograniczony capture, bez przekazywania sekretów MCP
do procesu mkcert. Nie instalujemy mkcert, nie generujemy CA ani nie wykonujemy
`mkcert -install`. Standardowy istniejący DDEV/mkcert nie wymaga lokalnego wpisu
CA/Compose, jeśli jego lokalizacja jest dostępna w środowisku klienta AI.

Launcher sam dołącza `compose.ca.yaml` przed oryginalną listą Compose, bez
dublowania ręcznego wpisu. Montuje osobno tylko wybrany publiczny plik RO,
nigdy cały CAROOT ani `rootCA-key.pem`. System/NSS Chromium i
`NODE_EXTRA_CA_CERTS` są konfigurowane wyłącznie w kontenerze; TLS pozostaje
włączony. Brak automatycznego CA nie blokuje publicznych stron. Ustawiony CAROOT
bez `rootCA.pem` kończy wykrywanie bez fallbacku; błąd/timeout/wadliwy wynik mkcert
daje krótką diagnostykę i brak autoimportu. Nieprawidłowa jawna ścieżka lub
wadliwa zawartość istniejącego wybranego CA zatrzymuje start.

Nie skanujemy HOME ani firmowych/systemowych trust stores i nie zmieniamy
zaufania hosta. Niestandardowy CA wymaga jawnego publicznego pliku przez
`AI_PROFILE_PLAYWRIGHT_CA_CERT`; nie obiecujemy zero-config dla nieznanego
firmowego CA. Po zmianie środowiska/ustawień zrestartuj klienta/MCP.

Uwierzytelniony audyt może użyć `AI_PROFILE_PLAYWRIGHT_STORAGE_STATE` wskazującego
opcjonalny storage state cookies/localStorage. Użytkownik tworzy go samodzielnie
w katalogu projektu, w formacie Playwright `{"cookies":[],"origins":[]}`,
z prawami `0600`; plik jest montowany RO i nie należy do raportów ani stanu instalacji.

HTTP Basic Auth jest opcjonalny: samodzielnie utwórz `<PROJECT>/base-auth.json`
z wymaganymi `username`, `password` i `origin`, o prawach `0600` lub bardziej
restrykcyjnych, bez symlinków. Przykład (fikcyjne wartości):

```json
{"username":"przykladowy-login","password":"fikcyjne-haslo","origin":"https://serwis.example:8443"}
```

Po utworzeniu pliku ustaw uprawnienia, uruchamiając w docelowym projekcie:

```sh
chmod 600 base-auth.json
```

W `origin` podaj protokół, host i opcjonalny port, bez ścieżki ani końcowego `/`,
np. `https://serwis.example:8443`.

Launcher automatycznie dołącza rozszerzenie; bez pliku nie włącza Basic Auth.
Nie commituj pliku ani nie udostępniaj go przez HTTP.
Instalacja/update/switch dodaje tylko regułę `/base-auth.json`
do `.gitignore`; nie odśledza już wersjonowanego sekretu. Reguła i prywatny plik
pozostają po uninstall. JSON jest montowany
tylko do odczytu. Entrypoint przekazuje dane do
`browser.contextOptions.httpCredentials`; origin ogranicza ich użycie do
wskazanego serwisu. Nie zapisuj sekretów w Git ani raportach. Rozszerzenie działa
razem z CA i storage state. Jawny `AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE` ma
pierwszeństwo; brak jawnie wskazanego pliku zatrzymuje start. Format pliku,
opcjonalne nadpisanie lokalizacji i
procedura restartu MCP po błędzie uwierzytelnienia są w
[instrukcji Basic Auth](images/playwright/README.md#http-basic-auth).

Raport, findings JSON, raw scans i screenshoty skill zapisuje w
`.ai-profiles/reports/<timestamp>/`. Compose przekazuje do kontenera tylko jawnie
zadeklarowane zmienne, nie całe środowisko hosta.

Dla jawnego `filename` preferuj `.ai-profiles/reports/<run>/...` albo odpowiadającą
bezwzględną ścieżkę hosta `<projekt>/.ai-profiles/reports/<run>/...`. Nazwy względne
rozwiązują się względem projektu z MCP roots, nie automatycznie względem katalogu
wyjściowego. Pominięcie `filename` używa skonfigurowanego `.ai-profiles/reports/`.
Sama nazwa `file.png` próbuje zapisać w projekcie tylko do odczytu i nie jest
obsługiwana. Nie używaj `/workspace/...` ani hostowego `/tmp/opencode`: nie są
aliasami projektu ani dodatkowymi mountami. Nie udostępniamy dowolnych katalogów
hosta. Osobne polecenie diagnostyczne Compose z jawną zmienną jest w
[instrukcji instalowanej](images/playwright/README.md#diagnostyka).

Obraz obsługuje `linux/amd64` i `linux/arm64`; pozostałe architektury są
odrzucane. Zakres rzeczywistych testów należy podawać oddzielnie dla każdej
architektury; testy obrazu są w `mcp/images/playwright/tests/`.

Runner `tests/run.sh` wymaga dodatkowo hostowego Node wyłącznie jako klienta
testowego MCP. Sprawdza entrypoint, a następnie instalację przez `make install`
i wspólny adapter uruchomiony z `/`, z prawdziwymi MCP roots. Przeglądarka i axe
pozostają w kontenerze. Test używa ścieżki projektu ze spacjami i przecinkiem,
weryfikuje artefakty także na hoście oraz próby nadpisania metadanych adaptera.
Scenariusz sieciowy potwierdza rzeczywiste mounty RO i ich zgodność z plikami
hosta VM oraz hostowy loopback bez lokalnej konfiguracji. Syntetyczny resolver
na `127.203.77.53:53` obsługuje wyłącznie `app.corp.test`; Chromium odwiedza tę
domenę przez testowy overlay, bez zmiany `/etc/resolv.conf` VM. To dowód
„syntetyczny resolver + host networking”, nie weryfikacja firmowego VPN.
Scenariusz CA tworzy syntetyczny CAROOT poza projektem: prawdziwe Chromium ufa
wykrytemu `rootCA.pem`, odrzuca obcy CA i zachowuje współdziałanie TLS, Basic Auth
oraz storage state. Inspect potwierdza oddzielny mount publicznego pliku RO,
bez katalogu CAROOT/klucza; sentinel klucza i credentials nie trafiają do
protokołu, logów, stanu ani raportów. Test obejmuje brak i wadliwą zawartość CA.

Timeouty: Codex dostaje startup/tool timeouty w sekundach. Claude dostaje
`env.MCP_TIMEOUT` w milisekundach (wpisany automatycznie, jeśli brak jawnej
wartości); jawna, zbyt mała wartość jest zachowana i zgłaszana. Fazy katalogu
nie mają odpowiednika w każdym kliencie, więc są odrzucane zamiast po cichu
zmieniania znaczenia.

## Walidacja

`make validate` sprawdza rejestr i wszystkie profile, w tym spójność sumy
`mcp/example.json`. `make validate PROFILE=<nazwa>` sprawdza wybrany profil,
jego skille wspólne i profilowe oraz wybrane definicje MCP, ale nie pozostałe
profile ani niewybrane wpisy rejestru.
