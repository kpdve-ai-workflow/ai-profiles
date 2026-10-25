# Playwright MCP — przygotowanie w projekcie

Ten katalog jest dostarczany przez profil React lub WordPress. `make install`
kopiuje konfigurację; nie uruchamia Dockera, npm ani budowy obrazu.
Z repozytorium instalatora użyj
`make install CLIENT={{AI_PROFILE_CLIENT}} PROFILE=react PROJECT=/sciezka/do/projektu`
(lub `PROFILE=wordpress`). Znaczniki `{{AI_PROFILE_CLIENT}}`, `{{AI_PROFILE_MCP}}`
i `{{AI_PROFILE_LOCAL}}` są rozwijane podczas instalacji dla wybranego klienta.

## Pierwsze uruchomienie

Z katalogu głównego **docelowego projektu**, mając działający Docker i Compose:

```sh
docker build --tag ai-profiles-playwright:0.2.0 {{AI_PROFILE_MCP}}/playwright
```

Build pobiera publiczne zależności i instaluje Node, Playwright MCP, Chromium,
axe-core i biblioteki systemowe wyłącznie w obrazie. Nie potrzebujesz Node/npm
na hoście, tokena GHCR ani publikacji obrazu. Obraz jest współdzielony lokalnie
przez projekty korzystające z tej samej wersji.

Opcjonalnie przygotuj plik [Basic Auth](#http-basic-auth), następnie zrestartuj
klienta AI i zatwierdź jego natywne zgody projektu/MCP, jeśli wymagane.
Wspólny adapter `ai-profile-mcp` uruchamia
natywny `playwright-launcher`, który przygotowuje runtime i wykonuje
standardowe `docker compose run` przez stdio.
Nie wykonuj `compose up -d`: każdy klient otrzymuje własny kontener i sesję.
Konfiguracja nie buduje ani nie pobiera brakującego obrazu automatycznie.

Projekt jest montowany pod tą samą bezwzględną ścieżką co na hoście, tylko do
odczytu; jedynie `.ai-profiles/reports/` jest zapisywalne. Wspólny adapter ustawia
`AI_PROFILE_PROJECT_DIR` na kanoniczną ścieżkę zainstalowanego projektu przy
każdym uruchomieniu. Użytkownik nie konfiguruje tej zastrzeżonej zmiennej;
środowisko, dotenv ani lokalne mapowanie nie mogą jej zastąpić.
Launcher tworzy brakujące `.ai-profiles/reports/` jako bieżący użytkownik przy starcie, nie
podczas Docker build; kontener dopasowuje UID/GID do właściciela tego katalogu.
Istniejące raporty pozostają nietknięte. Zwykłe użycie nie wymaga edycji Compose
ani `local.json`; VPN i konfiguracja hosta pozostają poza zakresem narzędzia.
Jawne `HOST_UID` i `HOST_GID` pozostają opcją dla niestandardowych uprawnień.

### Ścieżki artefaktów

- Dla jawnych nazw używaj `.ai-profiles/reports/<run>/...`, np.
  `.ai-profiles/reports/2026-10-21/screenshot.png`. Względna nazwa jest rozwiązywana
  względem katalogu projektu z MCP roots, **nie** katalogu wyjściowego.
- Odpowiadająca bezwzględna ścieżka hosta
  `<projekt>/.ai-profiles/reports/<run>/...` działa identycznie w kontenerze.
- Pominięcie `filename` korzysta ze skonfigurowanego katalogu `.ai-profiles/reports/`.
- Sama nazwa `file.png` wskazuje główny katalog projektu, który jest tylko do
  odczytu: taki zapis nie jest obsługiwany. Nie używaj `/workspace/...` ani nie
  zakładaj, że hostowe `/tmp/opencode` jest dostępne dla Playwright.

Nie zmieniamy reguł nazw upstream ani granic dostępu MCP. Kontener nie otrzymuje
dostępu do dowolnych katalogów hosta.

## Lokalne ustawienia

### Sieć lokalnego Linux Docker Engine

Domyślnie kontener korzysta z sieci `host`: `localhost` oznacza komputer,
na którym działa lokalny Docker Engine. Launcher automatycznie dołącza
`compose.host.yaml`, montując istniejące `/etc/hosts` i `/etc/resolv.conf`
tylko do odczytu (`create_host_path: false`). Lokalna aplikacja oraz gotowe
mapowania nazw i resolver hosta nie wymagają wyboru sieci DDEV w `local.json`.

Narzędzie nie uruchamia ani nie konfiguruje VPN, nie zmienia DNS, tras,
firewalla czy plików hosta. Korzysta z już działającego środowiska; nie gwarantuje
zgodności z dowolną polityką firmowej sieci. Po zmianie środowiska hosta zrestartuj
MCP/klienta. Docker Desktop i zdalny Engine nie są objęte tym kontraktem.

Nieustawione, puste lub równe `host` `AI_PROFILE_PLAYWRIGHT_NETWORK` włącza
automatyczny host overlay. Jawny inny tryb (np. `bridge` lub sieć DDEV) go nie
otrzymuje: hostowy loopback DNS nie jest resolverem kontenera bridge.
Automatyczny overlay poprzedza oryginalną listę `COMPOSE_FILE`, więc jawne pliki
użytkownika zachowują priorytet; overlay już obecny na liście nie jest dublowany
ani przenoszony. Sama zmiana sieci we własnym Compose wymaga zadbania o zgodność
mountów — launcher nie analizuje znaczenia takiego pliku.

Przykład opcjonalnego nadpisania w `.ai-profiles/{{AI_PROFILE_CLIENT}}/local.json`
dla świadomie wybranej sieci DDEV (sprawdź jej nazwę):

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

W takim niestandardowym trybie używaj adresu aplikacji dostępnego z kontenera
(np. adresu DDEV), nie zakładaj dostępu do hostowego `localhost`.
Lokalny plik może również wskazać `env_file` ze ścieżką względem projektu.
Nie zapisuj sekretów w zarządzanych plikach.

### Zaufany CA i uwierzytelniony audyt

Launcher automatycznie wybiera istniejący publiczny CA w kolejności:

1. Jawny `AI_PROFILE_PLAYWRIGHT_CA_CERT` (ma pierwszeństwo).
2. `rootCA.pem` z niepustego `CAROOT` odziedziczonego przez klienta AI.
3. `rootCA.pem` w lokalizacji zwróconej przez `mkcert -CAROOT`, jeśli `mkcert`
   jest już na PATH. To wyłącznie odczytowe zapytanie, z limitem 5 sekund
   i ograniczonym wynikiem; nie instaluje ani nie generuje CA.

Dla standardowego istniejącego DDEV/mkcert nie wpisujesz CA ani `COMPOSE_FILE`
do `local.json`. Klient musi widzieć istniejące `CAROOT` lub `mkcert` na PATH;
po zmianie jego środowiska uruchom go ponownie. Launcher sam dołącza
`compose.ca.yaml`, bez dublowania ręcznie dodanego rozszerzenia. Oryginalna lista
Compose zachowuje kolejność i priorytet nad automatycznymi plikami.

Nie skanujemy HOME ani systemowych/firmowych trust stores. Montowany jest tylko
wybrany zwykły publiczny plik CA, osobno i RO — nigdy cały CAROOT ani
`rootCA-key.pem`. Import do systemu/NSS Chromium i `NODE_EXTRA_CA_CERTS` odbywa
się wyłącznie wewnątrz kontenera. Weryfikacja TLS pozostaje włączona.

Brak mkcert lub automatycznego `rootCA.pem` nie blokuje stron z publicznym CA.
Ustawione CAROOT bez `rootCA.pem` nie uruchamia dalszego wyszukiwania.
Timeout/błąd/nieprawidłowy wynik mkcert daje krótką diagnostykę i brak autoimportu.
Błędna jawna ścieżka CA albo istniejący wybrany plik z błędną zawartością
zatrzymuje start — bez wyłączania TLS.

Niestandardowy (np. firmowy) CA nadal wymaga jawnego publicznego źródła zaufania:
ustaw `AI_PROFILE_PLAYWRIGHT_CA_CERT` w
`.ai-profiles/{{AI_PROFILE_CLIENT}}/local.json`. Nie przekazuj klucza CA.
Przykład jawnego CA i opcjonalnego storage state, bez ręcznego Compose:

```json
{
  "mcp": {
    "playwright": {
      "environment": {
        "AI_PROFILE_PLAYWRIGHT_CA_CERT": ".ddev/ai-profile-ca.crt",
        "AI_PROFILE_PLAYWRIGHT_STORAGE_STATE": ".ai-profiles/.private/playwright-auth.json"
      }
    }
  }
}
```

Można wskazać tylko potrzebne wejście. Względne ścieżki są względem katalogu
głównego projektu; bezwzględne też są obsługiwane. Pliki muszą już istnieć.
Storage state automatycznie włącza `compose.auth.yaml` i jest montowany tylko do odczytu
i musi mieć prawa `0600` lub bardziej restrykcyjne. Trzymaj go poza raportami
i Gitem. Przed użyciem dodaj prywatne pliki do projektowego `.gitignore`.

Po zmianach lokalnych ustawień zrestartuj MCP. Po `make update CLIENT={{AI_PROFILE_CLIENT}} PROJECT=<projekt>` ponów jawny
`docker build`, jeśli zmienił się Dockerfile lub entrypoint. Aktualizacja profilu
nie przebudowuje i nie usuwa obrazów; uninstall zachowuje raporty i prywatne pliki.

### HTTP Basic Auth

Basic Auth skonfiguruj niezależnie od cookies i localStorage ze storage state.
Utwórz prywatny plik `.ai-profiles/.private/playwright-basic-auth.json` o prawach `0600`,
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

Instalacja/update/switch profilu z Playwright dopisuje do `.gitignore` wyłącznie
`/.ai-profiles/.private/playwright-basic-auth.json`. Przed wpisaniem sekretów potwierdź, że
plik nie jest już śledzony przez Git — sama reguła nie usuwa go z indeksu.
Zapisz JSON lokalnym edytorem z prawami `0600` lub bardziej restrykcyjnymi;
katalog `.ai-profiles/.private` i plik nie mogą być symlinkami. Nigdy nie zapisuj go w `.ai-profiles/reports/`.
Nie przekazuj hasła w poleceniu terminala, URL, czacie ani argumentach narzędzi.
Plik może też leżeć poza projektem; wtedy podaj jego bezwzględną ścieżkę.
Musi być czytelny dla UID/GID kontenera, zwykle właściciela `.ai-profiles/reports/`.

Brak pliku oznacza brak Basic Auth. Jeśli plik istnieje, launcher sam dołącza
`compose.basic-auth.yaml`; nie edytujesz Compose ani `local.json`.
Tylko dla niestandardowej lokalizacji możesz dodać do istniejącego
`{{AI_PROFILE_LOCAL}}` (zachowując inne wpisy):

```json
{
  "mcp": {
    "playwright": {
      "environment": {
        "AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE": ".ai-profiles/.private/inny-basic-auth.json"
      }
    }
  }
}
```

Jawna niepusta ścieżka ma pierwszeństwo; brak wskazanego pliku jest błędem,
nie fallbackiem. Względne ścieżki są względem projektu, bez `..` i symlinków.
CA i storage state można łączyć z Basic Auth bez ręcznej listy Compose.
Compose montuje prywatny plik tylko do odczytu. Entrypoint sprawdza prawa,
format i wymagany origin, po czym tworzy konfigurację
`browser.contextOptions.httpCredentials` w prywatnym katalogu tymczasowym
kontenera i przekazuje ją do MCP przez `--config`. Sekrety nie trafiają do
środowiska procesu ani argumentów CLI. Nie zapisuj pliku ani wygenerowanej
konfiguracji w raportach lub Git; nie odczytuj ich narzędziami agenta.

Po aktualizacji profilu przebuduj obraz poleceniem z pierwszej sekcji. Po każdej
zmianie danych lub konfiguracji zrestartuj połączenie Playwright MCP, aby powstał
nowy kontener i kontekst przeglądarki, następnie ponów nawigację.
Przy HTTP 401, monicie logowania lub `ERR_INVALID_AUTH_CREDENTIALS` sprawdź
istnienie pliku, uprawnienia, login/hasło i zgodność origin (także portu). Brakujące
lub odrzucone dane zgłoś użytkownikowi do lokalnego uzupełnienia; nie kopiuj ich
do raportów, logów ani czatu.

Dokumentacja: [HTTP authentication](https://playwright.dev/docs/network#http-authentication),
[httpCredentials i origin](https://playwright.dev/docs/api/class-browser#browser-new-context-option-http-credentials).

## Diagnostyka

- Brak obrazu: wykonaj powyższy `docker build` w projekcie.
- Brak uprawnień: sprawdź dostęp procesu klienta do lokalnego Dockera i projektu;
  nie uruchamiaj całego klienta przez `sudo`.
- Błąd przygotowania `.ai-profiles/reports/`: launcher tworzy katalog; istniejący plik lub
  symlink pod tą nazwą blokuje start. Nie usuwa ani nie przenosi danych użytkownika.
- Błąd `docker compose`: zainstaluj plugin Compose i uruchom Docker Engine.
- Błąd zapisu: sprawdź właściciela i uprawnienia katalogu raportów.
- Niestandardowy CA: wskaż publiczny plik przez `AI_PROFILE_PLAYWRIGHT_CA_CERT`
  w `{{AI_PROFILE_LOCAL}}`; nigdy klucz CA ani wyłączenie TLS.
- Stare jawne nadpisania: sprawdź komendę, sieć i `COMPOSE_FILE` w
  `{{AI_PROFILE_LOCAL}}`. Zachowują priorytet; aktualizacja nie usuwa ich automatycznie.
- Zmiana credentials, środowiska lub konfiguracji wymaga restartu MCP/klienta.

React 4.0.0 / WordPress 3.0.0 używają `.ai-profiles/reports/` i
`.ai-profiles/.private/`, bez migracji i fallbacku do starych lokalizacji.
Sieć pozostaje `host`. Uninstall zachowuje raporty, credentials i regułę
`.gitignore`. Prywatne pliki wskazywane jawnie też muszą być pod `.ai-profiles/`.

Sama konfiguracja ani pozytywny status profilu nie potwierdzają połączenia MCP.

Wyłącznie do samodzielnej diagnostyki Compose, poza adapterem, jawnie podaj ścieżkę
(z katalogu docelowego projektu):

```sh
AI_PROFILE_PROJECT_DIR="$(pwd -P)" docker compose --project-directory . \
  -f {{AI_PROFILE_MCP}}/playwright/compose.yaml config
```
