# Playwright MCP — przygotowanie w projekcie

Ten katalog jest dostarczany przez profil React lub WordPress. `make install`
kopiuje konfigurację; nie uruchamia Dockera, npm ani budowy obrazu.

## Pierwsze uruchomienie

Z katalogu głównego **docelowego projektu**, mając działający Docker i Compose:

```sh
mkdir -p reports/wcag
docker build --tag ai-profiles-playwright:0.1.0 .opencode/mcp/playwright
```

Build pobiera publiczne zależności i instaluje Node, Playwright MCP, Chromium,
axe-core i biblioteki systemowe wyłącznie w obrazie. Nie potrzebujesz Node/npm
na hoście, tokena GHCR ani publikacji obrazu. Obraz jest współdzielony lokalnie
przez projekty korzystające z tej samej wersji.

Następnie zrestartuj klienta AI. Wspólny adapter `ai-profile-mcp` uruchamia
standardowe `docker compose run` przez stdio, bez osobnego launchera Playwright.
Nie wykonuj `compose up -d`: każdy klient otrzymuje własny kontener i sesję.
Konfiguracja nie buduje ani nie pobiera brakującego obrazu automatycznie.

Projekt jest montowany pod tą samą bezwzględną ścieżką co na hoście, tylko do
odczytu; jedynie `reports/wcag` jest zapisywalne. Wspólny adapter ustawia
`AI_PROFILE_PROJECT_DIR` na kanoniczną ścieżkę zainstalowanego projektu przy
każdym uruchomieniu. Użytkownik nie konfiguruje tej zastrzeżonej zmiennej;
środowisko, dotenv ani lokalne mapowanie nie mogą jej zastąpić.
Katalog raportów utwórz jako swój użytkownik, nie przez `sudo`:
kontener dopasowuje UID/GID do jego właściciela. W razie potrzeby ustaw `HOST_UID`
i `HOST_GID` przez lokalne `environment` lub `env_file` adaptera.

### Ścieżki artefaktów

- Dla jawnych nazw używaj `reports/wcag/<run>/...`, np.
  `reports/wcag/2026-10-21/screenshot.png`. Względna nazwa jest rozwiązywana
  względem katalogu projektu z MCP roots, **nie** katalogu wyjściowego.
- Odpowiadająca bezwzględna ścieżka hosta
  `<projekt>/reports/wcag/<run>/...` działa identycznie w kontenerze.
- Pominięcie `filename` korzysta ze skonfigurowanego katalogu `reports/wcag`.
- Sama nazwa `file.png` wskazuje główny katalog projektu, który jest tylko do
  odczytu: taki zapis nie jest obsługiwany. Nie używaj `/workspace/...` ani nie
  zakładaj, że hostowe `/tmp/opencode` jest dostępne dla Playwright.

Nie zmieniamy reguł nazw upstream ani granic dostępu MCP. Kontener nie otrzymuje
dostępu do dowolnych katalogów hosta.

## Lokalne ustawienia

Przykład `.opencode/ai-profile.local.json` dla sieci DDEV:

```json
{
  "mcp": {
    "playwright": {
      "environment": {
        "AI_PROFILE_PLAYWRIGHT_NETWORK": "ddev_default"
      }
    }
  }
}
```

Używaj adresu aplikacji dostępnego z kontenera (np. adresu DDEV), nie `localhost`
hosta. Lokalny plik może również wskazać `env_file` ze ścieżką względem projektu.
Nie zapisuj sekretów w zarządzanych plikach.

### Zaufany CA i uwierzytelniony audyt

Opcjonalne pliki Compose włączaj przez `COMPOSE_FILE` w lokalnym `environment`.
Przykład z obydwoma rozszerzeniami:

```json
{
  "mcp": {
    "playwright": {
      "environment": {
        "COMPOSE_FILE": ".opencode/mcp/playwright/compose.yaml:.opencode/mcp/playwright/compose.ca.yaml:.opencode/mcp/playwright/compose.auth.yaml",
        "AI_PROFILE_PLAYWRIGHT_NETWORK": "ddev_default",
        "AI_PROFILE_PLAYWRIGHT_CA_CERT": ".ddev/ai-profile-ca.crt",
        "AI_PROFILE_PLAYWRIGHT_STORAGE_STATE": ".private/playwright-auth.json"
      }
    }
  }
}
```

Można włączyć tylko potrzebne rozszerzenie. Ścieżki są względne katalogu głównego
projektu; pliki muszą już istnieć. CA trafia do magazynów zaufania systemu i Chromium,
bez wyłączania weryfikacji TLS. Storage state jest montowany tylko do odczytu
i musi mieć prawa `0600` lub bardziej restrykcyjne. Trzymaj go poza raportami
i Gitem. Przed użyciem dodaj prywatne pliki do projektowego `.gitignore`.

Po zmianach lokalnych ustawień zrestartuj MCP. Po `make update CLIENT={{AI_PROFILE_CLIENT}} PROJECT=<projekt>` ponów jawny
`docker build`, jeśli zmienił się Dockerfile lub entrypoint. Aktualizacja profilu
nie przebudowuje i nie usuwa obrazów; uninstall zachowuje raporty i prywatne pliki.

### HTTP Basic Auth

Basic Auth skonfiguruj niezależnie od cookies i localStorage ze storage state.
Utwórz prywatny plik `.private/playwright-basic-auth.json` o prawach `0600`,
zastępując przykładowe wartości lokalnie:

```json
{
  "username": "LOGIN",
  "password": "HASLO",
  "origin": "https://serwis.example:8443"
}
```

Wszystkie trzy pola są wymagane. `origin` zawiera protokół HTTP/HTTPS, domenę
i port serwisu (port domyślny można pominąć), bez ścieżki, końcowego `/`, query
ani fragmentu. Musi odpowiadać adresowi używanemu przez przeglądarkę w kontenerze.
Ogranicza wysyłanie danych HTTP Auth do tego origin; przekierowanie na inny
origin nie otrzyma tych danych.

Przed zapisaniem prawdziwych danych dodaj `.private/` oraz
`.opencode/ai-profile.local.json` do `.gitignore` projektu. Utwórz katalog z prawami
`0700` i pusty plik z prawami `0600`, następnie uzupełnij go w lokalnym edytorze.
Nie przekazuj hasła w poleceniu terminala, URL, czacie ani argumentach narzędzi.
Plik może też leżeć poza projektem; wtedy podaj jego bezwzględną ścieżkę.
Musi być czytelny dla UID/GID kontenera, zwykle właściciela `reports/wcag`.

Do istniejącego `.opencode/ai-profile.local.json` dodaj rozszerzenie i ścieżkę:

```json
{
  "mcp": {
    "playwright": {
      "environment": {
        "COMPOSE_FILE": ".opencode/mcp/playwright/compose.yaml:.opencode/mcp/playwright/compose.basic-auth.yaml",
        "AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE": ".private/playwright-basic-auth.json"
      }
    }
  }
}
```

Jeśli używasz już CA lub storage state, zachowaj ich zmienne oraz dopisz
`compose.basic-auth.yaml` do istniejącej listy `COMPOSE_FILE`. Rozszerzenia można
łączyć. Compose montuje prywatny plik tylko do odczytu. Entrypoint sprawdza prawa,
format i wymagany origin, po czym tworzy konfigurację
`browser.contextOptions.httpCredentials` w prywatnym katalogu tymczasowym
kontenera i przekazuje ją do MCP przez `--config`. Sekrety nie trafiają do
środowiska procesu ani argumentów CLI. Nie zapisuj pliku ani wygenerowanej
konfiguracji w raportach lub Git; nie odczytuj ich narzędziami agenta.

Po aktualizacji profilu przebuduj obraz poleceniem z pierwszej sekcji. Po każdej
zmianie danych lub konfiguracji zrestartuj połączenie Playwright MCP, aby powstał
nowy kontener i kontekst przeglądarki, następnie ponów nawigację.
Przy HTTP 401, monicie logowania lub `ERR_INVALID_AUTH_CREDENTIALS` sprawdź
włączenie rozszerzenia, istnienie pliku, uprawnienia i zgodność origin. Brakujące
lub odrzucone dane zgłoś użytkownikowi do lokalnego uzupełnienia; nie kopiuj ich
do raportów, logów ani czatu.

Dokumentacja: [HTTP authentication](https://playwright.dev/docs/network#http-authentication),
[httpCredentials i origin](https://playwright.dev/docs/api/class-browser#browser-new-context-option-http-credentials).

## Diagnostyka

- Brak obrazu: wykonaj powyższy `docker build` w projekcie.
- Brak `reports/wcag`: utwórz katalog poleceniem `mkdir -p reports/wcag`.
- Błąd `docker compose`: zainstaluj plugin Compose i uruchom Docker Engine.
- Błąd zapisu: sprawdź właściciela i uprawnienia katalogu raportów.

Sama konfiguracja ani pozytywny status profilu nie potwierdzają połączenia MCP.

Wyłącznie do samodzielnej diagnostyki Compose, poza adapterem, jawnie podaj ścieżkę
(z katalogu docelowego projektu):

```sh
AI_PROFILE_PROJECT_DIR="$(pwd -P)" docker compose --project-directory . \
  -f .opencode/mcp/playwright/compose.yaml config
```
