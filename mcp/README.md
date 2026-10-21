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
takie serwery konfiguruj niezależnie w OpenCode. Aktualizacja ostrzega o zmianie
upstream pól, które lokalnie nadpisano, bez ujawniania ich wartości. Nadpisania
niepasujące do aktualnego profilu powodują błąd, a nie ciche pominięcie.
Nie edytuj wygenerowanych wpisów `opencode.json` — ręczne zmiany blokują update.

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
środowiska OpenCode. Zdalne MCP uruchamia natywnie OpenCode i nie używają
`env_file`. Osobny `ai-profile-mcp.json` nie jest wymagany; update usuwa jego
wcześniej zarządzaną kopię. Adapter działa bez Python, Cargo i checkoutu;
zależności wybranego serwera muszą być dostępne osobno.
Po zmianie pliku env lub mapowania zrestartuj MCP;
po zmianie ustawień OpenCode wykonaj update i zrestartuj sesję.

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

## Walidacja

`make validate` sprawdza rejestr i wszystkie profile, w tym spójność sumy
`mcp/example.json`. `make validate PROFILE=<nazwa>` sprawdza wybrany profil,
jego skille wspólne i profilowe oraz wybrane definicje MCP, ale nie pozostałe
profile ani niewybrane wpisy rejestru.
