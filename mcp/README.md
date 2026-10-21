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

Profile WordPress i React wybierają serwer `playwright` oraz skill
`wcag-audit`. Pozostałe profile nie otrzymują przeglądarki. Launcher uruchamia
pojedynczy prebuilt image przez Docker `-i` bez TTY, projekt montuje tylko do
odczytu, a `reports/wcag` zapisywalnie pod tą samą ścieżką hosta. Każda sesja
klienta ma własny kontener i sesję przeglądarki. Brak obrazu wywołuje pull,
nigdy lokalny build.

Deskryptor `mcp/launchers/playwright/image.json` wskazuje
`ghcr.io/kpdve-ai-workflow/playwright:0.1.0`. Pierwszy release nie został
opublikowany, więc Playwright nie jest jeszcze dostępny w projektach.

### Ręczne wydanie obrazu

Źródła są w `mcp/images/playwright/`. Aktualizując zależności, przypnij dokładne
wersje `@playwright/mcp` i `axe-core`, wygeneruj committed `package-lock.json`,
sprawdź wersję Playwright zależną od MCP, a następnie zbuduj, przetestuj i
opublikuj nowy tag obrazu. Nie nadpisuj istniejących wersji. Pierwsze wydanie:

```sh
docker login ghcr.io
docker buildx build --platform linux/amd64,linux/arm64 \
  --tag ghcr.io/kpdve-ai-workflow/playwright:0.1.0 \
  --push mcp/images/playwright
docker buildx imagetools inspect ghcr.io/kpdve-ai-workflow/playwright:0.1.0
```

Przed publikacją sprawdź dostępność Chromium na obu architekturach i test MCP.
Zapisz digest z manifestu w notatkach release. Po potwierdzeniu publikacji
deskryptor `image.json` jest źródłem runtime tagu. Kolejne wydanie publikuje
nowy niezmienny tag, a następnie zmienia deskryptor i wymaga `make update` w
projektach. Obecnie descriptor nie zawiera digestu, bo nie ma opublikowanego
digestu do wpisania. Nie ma workflow publikującego; publikacja jest ręczna.

Obraz ma przypięty Node `22.16.0-bookworm-slim`, dokładny lockfile i instaluje
Chromium zależne od pakietu Playwright w czasie budowy. Skrypt axe jest
przekazywany przez opcję MCP `--init-script`, więc ładuje się po nawigacji i w
nowych stronach, nie wymaga CDN ani instalacji w aplikacji.

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
`AI_PROFILE_PLAYWRIGHT_CA_CERT` wskazuje plik CA. Launcher montuje ten plik
oddzielnie, a kontener dodaje go do systemowego magazynu CA przed startem
Chromium; weryfikacja TLS pozostaje włączona.

Uwierzytelniony audyt może użyć `AI_PROFILE_PLAYWRIGHT_STORAGE_STATE` wskazującego
prywatny plik storage state o prawach `0600`. Plik jest montowany tylko do
odczytu pod oddzielną ścieżką, nie należy do raportów ani stanu instalacji.
Raport, findings JSON, raw scans i screenshoty skill zapisuje w
`reports/wcag/<timestamp>/`. Launcher nie przekazuje całego host env i nie
montuje Docker socketu.

Obraz deklaruje `linux/amd64` i `linux/arm64`; pozostałe architektury są
odrzucane. Potwierdź Chromium uruchomieniowo na obu przed release.

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
