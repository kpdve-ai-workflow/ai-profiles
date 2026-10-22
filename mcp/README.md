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
`.opencode/ai-profile.local.json`) do OpenCode (`opencode.json`), Codex
(`.codex/config.toml`, zachowywany przez `toml_edit`) i Claude Code (`.mcp.json`).
CLI klientów nie są wymagane do przygotowania konfiguracji. Wpisy są śledzone
oddzielnie w stanie instalacji; kolizje i ręczne zmiany zarządzanych wpisów
blokują aktualizację. Klienci nadal kontrolują zaufanie projektu, zatwierdzanie
MCP i OAuth. Status potwierdza przygotowaną konfigurację, nie runtime connection.

## Lokalne ustawienia projektu

`.opencode/ai-profile.local.json` jest plikiem użytkownika/projektu. Menedżer go
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

`env_file` jest opcjonalną ścieżką względną projektu, bez `..` i symlinków,
dostępna tylko dla lokalnych MCP. Przechowuj pliki sekretów poza Gitem, dodając
je do projektowego `.gitignore`; konfiguruj sekrety przez referencje środowiskowe,
nie wpisując ich w JSON. Kolejność wartości to środowisko odziedziczone, wskazany
plik env, a następnie mapowanie `environment`. Parser pliku env obsługuje
pojedyncze `KEY=value`, cudzysłowy, komentarze i `export`; nie wykonuje powłoki,
nie interpoluje `$VAR`/`${VAR}` i nie obsługuje wartości wieloliniowych.
Brak wymaganego pliku lub zmiennej zatrzymuje start MCP.

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

Instalator dostarcza natywne binarium Rust `.opencode/ai-profile-mcp` i nadaje
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

Opcjonalne zarządzane pliki runtime umieszczaj w `mcp/launchers/<nazwa>/`.
Instalowane są do `.opencode/mcp/<nazwa>/` wyłącznie projektom profili wybierających
serwer i podlegają kontroli hashy, konfliktów oraz usuwaniu jak inne zarządzane
pliki. Nie przechowuj w nich sekretów, cache ani artefaktów budowy. Szczegóły
konkretnej integracji znajdują się w dokumentacji właściwego profilu.

Launcher Rust `.opencode/mcp/mariadb/mariadb-launcher` jest kopiowany z
lokalnego release builda. Historyczny `launch.py` pozostaje w repozytorium;
nie jest instalowany. Launcher buduje przypięte źródła upstream dopiero przy pierwszym
włączonym uruchomieniu MCP, nie podczas instalacji profilu. Weryfikuje SHA-256
archiwum i buduje obraz Docker; szczegółowe wymagania, konfiguracja i ostrzeżenia
dotyczące SQL znajdują się w [instrukcji WordPress](../profiles/wordpress/README.md).

## Playwright i audyty WCAG

Pełny test repozytorium uruchamia `make test-playwright` w trwałej VM Debian 13
amd64 zarządzanej przez Incus (Docker i Node.js 24 LTS wewnątrz gościa).
Etap zainstalowanego adaptera wymaga Node na maszynie wykonującej skrypt, czyli
w VM; produkcyjne użycie MCP opisane poniżej go nie wymaga na hoście.
`network_mode: host` w integracji oznacza sieć VM, nie fizycznego hosta.
Wymagania, cache i eksport raportów: [testowanie](../docs/testing.md).

Profile WordPress i React wybierają serwer `playwright` oraz skill
`wcag-audit`. Pozostałe profile nie otrzymują konfiguracji przeglądarki.
`make install PROFILE=<profil> PROJECT=<projekt>` dostarcza do projektu kompletny
kontekst budowy i konfigurację Compose w `.opencode/mcp/playwright/`. Instalacja
nie wymaga Dockera i nie buduje ani nie pobiera obrazu.

### Przygotowanie w repozytorium docelowym

Z katalogu głównego projektu, mając działający Docker Engine i plugin Compose:

```sh
mkdir -p reports/wcag
docker build --tag ai-profiles-playwright:0.1.0 .opencode/mcp/playwright
```

Następnie zrestartuj klienta AI. Jak dla pozostałych lokalnych MCP, wspólny
adapter `ai-profile-mcp` odczytuje konfigurację i uruchamia jej komendę — tutaj
standardowe `docker compose run` bez TTY. Nie ma dedykowanego launchera Playwright,
publikacji GHCR, tokenów ani automatycznego buildu/pull obrazu. Nie uruchamiaj
`compose up -d`: każdy klient ma własny kontener i sesję przeglądarki.

Compose montuje projekt pod jego rzeczywistą bezwzględną ścieżką hosta, tylko
do odczytu, a `reports/wcag` pod tą samą ścieżką zapisywalnie. Ścieżkę dostarcza
wspólny adapter przez `AI_PROFILE_PROJECT_DIR`, bez konfiguracji użytkownika.
Katalog raportów musi wcześniej utworzyć użytkownik projektu;
kontener dopasowuje do niego UID/GID. Nie montuje Docker socketu. Opcjonalne
`HOST_UID` i `HOST_GID` pozwalają jawnie wybrać użytkownika.

Dockerfile deklaruje dokładne wersje `@playwright/mcp` i `axe-core`, bez npm
manifestu i lockfile w repozytorium. Node, npm dependencies, Chromium i biblioteki
systemowe są instalowane wyłącznie wewnątrz obrazu. Skrypt axe jest ładowany przez
`--init-script` również po kolejnych nawigacjach. Nie potrzeba hostowego Node/npm,
cache przeglądarki ani CDN. Przypięte wersje bezpośrednie nie gwarantują identycznych
zależności przechodnich w przyszłych rebuildach.

Po aktualizacji dostarczonej receptury użytkownik jawnie ponawia build w projekcie.
Aktualizacja i uninstall nie zarządzają lokalnymi obrazami ani raportami.
Szczegółowa instrukcja jest [dostarczana razem z konfiguracją](images/playwright/README.md).

### Sieć, certyfikaty i artefakty

W `.opencode/ai-profile.local.json` ustawiaj `environment` dla `playwright`.
Przykład konfiguracji lokalnej DDEV (sprawdź nazwę sieci u siebie):

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

Używaj URL dostępnego w tej sieci, np. `https://<projekt>.ddev.site`, nie
`localhost` hosta. Dla lokalnego HTTPS
`AI_PROFILE_PLAYWRIGHT_CA_CERT` wskazuje plik CA. Włącz dodatkowo `compose.ca.yaml`
przez lokalne `environment.COMPOSE_FILE`, zgodnie z dostarczoną instrukcją.
Compose montuje plik oddzielnie, a kontener dodaje go do magazynów CA systemu i Chromium
przed startem Chromium; weryfikacja TLS pozostaje włączona.

Uwierzytelniony audyt może użyć `AI_PROFILE_PLAYWRIGHT_STORAGE_STATE` wskazującego
prywatny plik storage state o prawach `0600`. Włącz `compose.auth.yaml` przez
`COMPOSE_FILE`. Plik jest montowany tylko do odczytu pod oddzielną ścieżką,
nie należy do raportów ani stanu instalacji.

HTTP Basic Auth włączaj oddzielnie przez `compose.basic-auth.yaml` oraz
`AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE` w lokalnym `environment`. Prywatny JSON
z wymaganymi `username`, `password` i `origin` ma prawa `0600` i jest montowany
tylko do odczytu. Entrypoint przekazuje dane do
`browser.contextOptions.httpCredentials`; origin ogranicza ich użycie do
wskazanego serwisu. Nie zapisuj sekretów w Git ani raportach. Rozszerzenie działa
razem z CA i storage state. Format pliku, przykład lokalnej konfiguracji i
procedura restartu MCP po błędzie uwierzytelnienia są w
[instrukcji Basic Auth](images/playwright/README.md#http-basic-auth).

Raport, findings JSON, raw scans i screenshoty skill zapisuje w
`reports/wcag/<timestamp>/`. Compose przekazuje do kontenera tylko jawnie
zadeklarowane zmienne, nie całe środowisko hosta.

Dla jawnego `filename` preferuj `reports/wcag/<run>/...` albo odpowiadającą
bezwzględną ścieżkę hosta `<projekt>/reports/wcag/<run>/...`. Nazwy względne
rozwiązują się względem projektu z MCP roots, nie automatycznie względem katalogu
wyjściowego. Pominięcie `filename` używa skonfigurowanego `reports/wcag`.
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
