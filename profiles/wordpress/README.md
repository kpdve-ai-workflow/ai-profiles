# Profil WordPress: Novamira + Figma + MariaDB

Jedna instalacja profilu, następnie konfiguracja połączeń dla Twojego projektu.
Nie używamy Pi. Novamira korzysta z loginu i **hasła aplikacyjnego WordPressa**, bez OAuth.
Profil wybiera `novamira`, `figma` i `mariadb` w swoim `mcp.json`; ich jedyne definicje
znajdują się w `mcp/servers/`. Figma jest współdzielona z profilem React,
ale każdy projekt zachowuje własne lokalne ustawienia i połączenie.

## 1. Zainstaluj profil

Z katalogu repozytorium `ai-profiles`:

```bash
make install PROFILE=wordpress PROJECT=/sciezka/do/projektu
```

Podaj istniejący katalog repozytorium WordPressa. Jeśli projekt ma już profil,
użyj `make update PROJECT=...` dla zainstalowanego WordPress albo
`make switch PROFILE=wordpress PROJECT=...` dla innego profilu.

Instalacja dostarcza:

- wspólne i wordpressowe skille do `.opencode/skills/`;
- definicje MCP `novamira`, `figma` i `mariadb` w zarządzanej konfiguracji OpenCode;
- adapter `.opencode/ai-profile-mcp.py`, który uruchamia most Novamira;
- launcher MariaDB `.opencode/mcp/mariadb/launch.py`, który buduje i uruchamia
  oficjalny serwer MCP w Dockerze;
- tę instrukcję wraz z przykładami konfiguracji w `.opencode/ai-profile/README.md`;
- skill SQL w `.opencode/skills/mcp-mariadb/`.

**Novamira i MariaDB są początkowo wyłączone**, aby instalacja działała bez sekretów
i uruchomionego WordPressa. Figma jest skonfigurowana, ale wymaga autoryzacji.
Instalator nie pobiera ani nie instaluje pluginów WordPressa, Node czy npm.
Skille dotyczące Sage/ACF stosuj tylko do pasującej architektury projektu;
zawarte w nich przykłady nie tworzą automatycznie pól, treści ani konfiguracji motywu.

## 2. Przygotuj środowisko

- Python 3.10+ dla naszego adaptera oraz Git dla menedżera profili.
- OpenCode dostępny na hoście, na którym pracujesz.
- Node.js **24.21.0 lub nowszy z linii 24** i npm z dostępnym `npx` na tym samym
  hoście. Profil używa `NODE_OPTIONS=--use-system-ca`, więc potrzebna jest wersja
  Node obsługująca tę opcję. Sprawdź `node --version` i `npx --version`.
- Działający WordPress przez HTTPS. Przy DDEV uruchom projekt przez `ddev start`.
- Certyfikat HTTPS zaufany przez system hosta i Node. Nie wyłączaj weryfikacji TLS.
- Dostęp do rejestru npm przy pierwszym uruchomieniu mostu. `npx` pobiera
  przypięty `@automattic/mcp-wordpress-remote@0.4.0` do swojego cache;
  nie jest to instalacja pluginu WordPressa ani globalnego pakietu.

Punktem odniesienia jest Novamira **1.12.3**, WordPress **6.9+** i PHP **8.0+**
według audytu. Dla innej wersji sprawdź wymagania producenta. Historyczny audyt
`wordpress-config.md` nie jest konfiguracją tego projektu: nie kopiuj z niego
domen, kont ani ustawień Pi/Codex.

## 3. Zainstaluj plugin Novamira w WordPressie

1. Pobierz pełną paczkę ZIP z [novamira.ai/download](https://novamira.ai/download/).
   Wybierz wersję 1.12.3, jeśli odtwarzasz wersję z audytu. Główny przycisk
   pobierania może dostarczać nowszą wersję — sprawdź numer przed instalacją.
2. Otwórz panel **docelowej witryny**, np. `https://moj-projekt.ddev.site/wp-admin/`.
3. Wybierz **Wtyczki → Dodaj nową → Wyślij wtyczkę na serwer**, wskaż ZIP,
   kliknij **Zainstaluj**, a następnie **Włącz**.
4. W menu **Novamira → Configuration** włącz **AI Abilities**.

AI Abilities udostępniają funkcje WordPressa agentowi: np. odczyt treści i operacje
na danych. Nadal wymagane są poświadczenia i uprawnienia konta. Novamira wiąże
włączenie z domeną witryny; po zmianie domeny lub imporcie bazy sprawdź lokalne
URL WordPressa i ponownie włącz AI Abilities dla właściwej domeny.

Używaj pełnej paczki z zależnościami `vendor/`, nie automatycznego GitHub
„Source code ZIP”. Novamira zawiera adapter MCP — nie instaluj osobnego
WordPress MCP Adapter tylko na potrzeby tego profilu.

W multisite wybierz właściwy blog. Nie aktywuj sieciowo ani nie dodawaj kolejnych
blogów wyłącznie z powodu MCP. Jeden skonfigurowany URL wskazuje jedną witrynę.
Profil nie zakłada blog ID `1`, BIP ani konkretnej domeny.

## 4. Utwórz hasło aplikacyjne

1. Zaloguj się na konto WordPressa, którego ma używać OpenCode.
2. Otwórz **Użytkownicy → Twój profil**.
3. Znajdź **Hasła aplikacji / Application Passwords**.
4. Wpisz nazwę `OpenCode local` i kliknij **Dodaj nowe hasło aplikacji**.
5. Skopiuj wygenerowane hasło do prywatnego pliku z następnego kroku.
   WordPress pokazuje je tylko raz. Nie wysyłaj go agentowi ani do czatu.

To osobne hasło integracji, **nie Twoje hasło do wp-admin**. W Novamira 1.12.3
konto potrzebuje `manage_options` w single-site (zwykle administratora),
a w multisite statusu superadministratora sieci. Hasło dziedziczy uprawnienia
konta; sama nazwa hasła nie ogranicza dostępu do odczytu.

Jeśli sekcji nie ma, sprawdź HTTPS i ustawienia pluginów bezpieczeństwa,
które mogą wyłączać Application Passwords. Standardowe hasła aplikacyjne nie
mają czasowego wygaśnięcia, ale mogą zostać unieważnione lub zablokowane polityką.

## 5. Zapisz lokalne połączenie

W katalogu **projektu WordPressa**, przed wpisaniem sekretów, dodaj do `.gitignore`:

```gitignore
/.env.novamira
```

Jeśli plik z sekretami był już śledzony przez Git, samo `.gitignore` nie wystarczy.
Usuń go z indeksu i unieważnij ujawnione hasło. Nie zapisuj sekretów w przykładach
wewnątrz `.opencode/skills/`: są to zarządzane, wersjonowane pliki profilu.

Utwórz `.env.novamira` w katalogu głównym projektu i ustaw prawa **przed** wpisaniem hasła:

```bash
touch .env.novamira
chmod 600 .env.novamira
```

Uzupełnij go w edytorze:

```dotenv
WP_API_URL=https://moj-projekt.ddev.site/wp-json/mcp/novamira
WP_API_USERNAME=twoj-login-wordpress
WP_API_PASSWORD="wygenerowane haslo aplikacyjne"
```

Podaj pełny adres z `/wp-json/mcp/novamira`, **nie `/novamira-oauth`**,
oraz login konta, na którym wygenerowano hasło. Cudzysłowy pozwalają zachować spacje.
Plik nie jest skryptem powłoki — nie wpisuj do niego komend ani interpolacji `$VAR`.

Następnie utwórz `.opencode/ai-profile.local.json`:

```json
{
  "mcp": {
    "novamira": {
      "disabled": false,
      "env_file": ".env.novamira"
    }
  }
}
```

Jeśli ten plik już istnieje, **dodaj wpis `novamira` do istniejącego `mcp`**,
zachowując inne ustawienia. Nie zastępuj całego pliku przykładem.
Plik jest własnością projektu, a nie generowaną konfiguracją OpenCode.
`env_file` jest ścieżką względem projektu, bez `..` i symlinków.

Nazwę pliku env możesz zmienić: zmień jednocześnie `env_file` i wpis `.gitignore`.
Nie potrzebujesz ręcznego eksportowania zmiennych do sesji OpenCode.
Upewnij się, że serwer WWW blokuje dostęp HTTP do plików `.env*` i `.opencode/`.
Uprawnienia `600` i `.gitignore` nie zastępują konfiguracji serwera WWW.

## 6. Włącz połączenie w OpenCode

Z katalogu `ai-profiles` zsynchronizuj ustawienie `disabled`:

```bash
make update PROJECT=/sciezka/do/projektu
```

Zrestartuj OpenCode i w projekcie sprawdź `opencode mcp list` lub panel `/mcps`.
Novamira powinna się połączyć bez przeglądarki i bez autoryzacji OAuth:

```text
OpenCode → adapter ai-profiles → most Node → HTTPS + Application Password → Novamira
```

Adapter czyta plik env przy uruchomieniu MCP. Zmienne są przekazywane tylko do
procesu mostu; hasło nie trafia do argumentów komendy, generowanego `opencode.json`
ani stanu instalacji. `OAUTH_ENABLED=false` jest ustawione w definicji profilu.
Nie dodawaj `JWT_TOKEN`, alternatywnych nagłówków uwierzytelniania ani nadpisania
`OAUTH_ENABLED` w lokalnym środowisku mostu.

Poproś agenta o **discovery, odczyt schematu i odczyt ID bloga oraz adresu witryny**.
Potwierdź właściwy cel. Nie trzeba tworzyć strony testowej ani zmieniać ustawień.
Samo otwarcie endpointu w przeglądarce nie jest testem MCP: GET może zwracać `405`.

Po zmianie samego hasła/pliku env zrestartuj MCP. Po zmianie `disabled` lub innych
ustawień OpenCode wykonaj `make update` i restart sesji. Nie edytuj ręcznie
zarządzanych wpisów w `opencode.json`.

## 7. Połącz Figmę

Serwer `figma` korzysta z oficjalnego endpointu `https://mcp.figma.com/mcp`.
Nie wymaga aplikacji Figma Desktop ani hasła WordPressa.

1. W projekcie otwórz OpenCode i panel `/mcps`.
2. Wybierz `figma` i opcję logowania/autoryzacji.
3. Zaloguj się do właściwego konta Figma w przeglądarce i zatwierdź dostęp.
4. Sprawdź stan połączenia. Do zadania podaj link do konkretnej ramki lub węzła
   przez **Copy link to selection** w Figmie.

Figma używa własnego OAuth. **Wykluczenie OAuth dotyczy Novamiry**, nie Figmy.
Tokenów Figmy nie zapisuj w `.env.novamira` ani konfiguracji profilu.
Dostępność zależy również od uprawnień do pliku, planu, limitów i obsługi klienta
przez Figmę. Sam zapis konfiguracji nie potwierdza udanej autoryzacji.

Jeśli chwilowo nie potrzebujesz Figmy, dodaj do lokalnego `mcp` wpis
`"figma": {"disabled": true}`, wykonaj update i restart OpenCode.

## 8. Połącz MariaDB — pełny SQL

MariaDB MCP jest niezależny od Novamiry. Łączy się bezpośrednio z bazą SQL;
nie wymaga pluginu WordPressa, OAuth, usługi HTTP MCP ani kluczy do modeli AI.

### Wymagania i pierwsza budowa

- Host Linux, działający Docker CLI i Linux Docker Engine dostępny dla procesu
  OpenCode. Nie montujemy gniazda Docker wewnątrz kontenera MCP.
- Uruchomiona baza projektu, np. przez DDEV. Profil jej nie uruchamia i nie tworzy.
- Konto MariaDB z **pełnymi uprawnieniami do docelowej bazy**, w tym do żądanych
  operacji DDL/DML. Nie wymuszamy konta read-only; instalator nie nadaje sam grantów.
- Przy pierwszym uruchomieniu: dostęp do GitHub, rejestrów obrazu bazowego
  i zależności oraz kilka GB wolnego miejsca. Pełne zależności upstream obejmują
  biblioteki ML, mimo że embeddings nie są w tej integracji włączone.

`make install` dostarcza launcher bez pobierania źródeł i bez budowania obrazu.
Po skonfigurowaniu i włączeniu MCP launcher:

1. Sprawdza wymagane zmienne, następnie obecność obrazu Docker.
2. Jeśli obrazu brak, pobiera archiwum oficjalnego
   [MariaDB/mcp](https://github.com/MariaDB/mcp) w wersji 0.2.6, commit
   `64876fb1a4cb81bf51564c46eb60401097c067cc`.
3. Weryfikuje SHA-256 archiwum:
   `0730bd3dd546bce18fb3cb4f940fd79980d6114b89909c72da1ebefc1d5fbbc0`.
4. Buduje z jego Dockerfile obraz
   `ai-profiles/mariadb-mcp:64876fb1a4cb81bf51564c46eb60401097c067cc`.
   Budowa nie otrzymuje zmiennych bazy ani prywatnych plików projektu jako kontekstu.
5. Uruchamia MCP przez stdio, przekazując poświadczenia przez środowisko,
   nie wartości w argumentach komendy. Nie publikuje portu HTTP MCP.

Nie potrzebujesz nowej komendy Make ani ręcznego klonowania źródeł.
Pierwsza budowa może trwać kilka–kilkadziesiąt minut; postęp trafia na stderr,
nie na strumień protokołu. Startup timeout tego MCP wynosi godzinę.
Przy kolejnych uruchomieniach i w innych projektach korzystających z tego samego
Docker Engine gotowy obraz jest współdzielony. Blokada budowy jest przechowywana
w `${XDG_CACHE_HOME:-~/.cache}/ai-profiles/mariadb/`.

Przypięte są źródła i ich checksum, nie wszystkie zależności przechodnie ani
obraz `python:3.11-slim` upstream. Nie jest to gwarancja bitowo identycznych rebuildów.
Obraz budujemy i uruchamiamy jako `linux/amd64`, zgodnie z obejściem upstream dla
awarii asyncmy na natywnym ARM64. Inna architektura hosta wymaga emulacji Docker.

### Połączenie per projekt

W głównym katalogu projektu dodaj do `.gitignore`:

```gitignore
/.env.mariadb
```

Utwórz plik i ustaw prawa przed wpisaniem hasła:

```bash
touch .env.mariadb
chmod 600 .env.mariadb
```

Przykład dla DDEV z portem bazy opublikowanym na hoście:

```dotenv
DB_HOST=127.0.0.1
DB_PORT=62335
DB_NAME=twoja_baza
DB_USER=twoj_uzytkownik_bazy
DB_PASSWORD="twoje haslo do bazy"
```

**62335 jest przykładem z konfiguracji WFOŚiGW**, nie globalnym domyślnym portem.
Sprawdź `host_db_port` w `.ddev/config.yaml` i dane połączenia po uruchomieniu DDEV
(np. `ddev describe`). Nazwę bazy i konto potwierdź dla konkretnego projektu.
Hasło aplikacyjne Novamiry nie jest hasłem MariaDB.

Domyślnie launcher stosuje `--network=host`, więc `127.0.0.1` oznacza host Linuxa.
Nie używaj tu nazwy `db`, która zwykle jest dostępna tylko w sieci kontenerów DDEV.
Nie przekierowuj połączenia na inną bazę przez zgadywanie portów.

Do istniejącego `.opencode/ai-profile.local.json` dodaj wpis `mariadb` wewnątrz `mcp`:

```json
{
  "mcp": {
    "mariadb": {
      "disabled": false,
      "env_file": ".env.mariadb"
    }
  }
}
```

**Zachowaj istniejące wpisy `novamira`, `figma` i inne ustawienia.** Przykłady
konfiguracji znajdują się wyłącznie w tej instrukcji, nie w skillach. Nazwę pliku env możesz
zmienić. Własne nazwy zmiennych mapujesz przez lokalne `environment`, np.
`"DB_PASSWORD": "{env:LOCAL_DB_PASSWORD}"`. Pozostałe pola środowiska są scalane.

Jeżeli potrzebujesz połączenia po sieci Docker zamiast portu hosta, ustaw
`MARIADB_MCP_NETWORK=nazwa_sieci`, `DB_HOST=nazwa_kontenera_bazy` i `DB_PORT=3306`
w tym samym pliku env. Najpierw potwierdź nazwy sieci/kontenera; profil ich nie
tworzy ani nie zmienia. Ścieżki do certyfikatów i mounty dla zdalnych baz TLS
wymagają osobnego dostosowania komendy — ten domyślny wariant dotyczy lokalnego DDEV.

Z katalogu `ai-profiles` wykonaj standardowe:

```bash
make update PROJECT=/sciezka/do/projektu
```

Zrestartuj OpenCode, sprawdź `/mcps` lub `opencode mcp list`, poczekaj na pierwszą
budowę. Do sprawdzenia połączenia wystarczy zapytanie
`SELECT DATABASE(), VERSION()` i odczyt schematu. Nie trzeba tworzyć testowych tabel.

### Uprawnienia i używanie SQL

Profil ustawia **`MCP_READ_ONLY=false` i `MCP_BLOCK_SENSITIVE_SHOW=false`**.
MCP dopuszcza odczyt, zapis, DDL i administracyjne polecenia SHOW. Ostateczne
możliwości wynikają z uprawnień konta MariaDB — te flagi nie przyznają grantów.
`DB_NAME` jest domyślną bazą połączenia, a nie granicą dostępu konta.

Nie ograniczamy tego połączenia do read-only. Pełny dostęp nie oznacza jednak
zgody na niezamówione kasowanie, migracje lub zmiany uprawnień. W multisite
jedna baza może zawierać tabele kilku blogów; przed zapisem ustal blog i prefiks.
Bezpośredni SQL omija hooki, walidację i cache WordPressa. Nie zastępuj nim
bez potrzeby operacji treściowych, które wykonuje Novamira.
Upstream blokuje wykonywanie wielu instrukcji w jednym zapytaniu; osobne
wywołania MCP mogą korzystać z różnych połączeń z puli.

Docker otrzymuje tylko jawnie wybrane zmienne bazy i flagi MCP, bez sekretów
Novamiry czy kluczy embeddingów. Administratorzy demona Docker mogą odczytać
środowisko kontenera — korzystaj z zaufanego lokalnego demona. Upstream loguje
SQL i parametry: nie umieszczaj sekretów w zapytaniach ani nie publikuj pełnych logów.
Serwer WWW musi blokować dostęp HTTP do `.env*` i `.opencode/`.

## 9. Aktualizacje, odinstalowanie i diagnostyka

```bash
make status PROJECT=/sciezka/do/projektu
make update PROJECT=/sciezka/do/projektu
make uninstall PROJECT=/sciezka/do/projektu
```

Te komendy zarządzają wyłącznie profilem i jego plikami. **Nie instalują,
nie aktualizują, nie wyłączają ani nie usuwają pluginu Novamira, danych WordPressa
czy hasła aplikacyjnego**. Zachowują lokalną konfigurację i plik env.
Tak samo nie zmieniają bazy MariaDB, kont i grantów ani nie usuwają pliku
`.env.mariadb`. Odinstalowanie usuwa zarządzany launcher, ale zostawia obraz
Docker i cache blokady, ponieważ mogą być używane przez inne projekty.
Zmiana przypiętego commita w przyszłej wersji launchera tworzy nowy tag obrazu;
nie zastępuje starego tagu ani nie wykonuje migracji SQL.
Plugin aktualizujesz osobno, po backupie i sprawdzeniu zgodności.
Po zakończeniu korzystania z integracji możesz samodzielnie unieważnić jej hasło.

| Objaw | Sprawdź |
| --- | --- |
| Novamira wyłączona | `disabled: false` w lokalnym pliku, potem update i restart |
| Brak zmiennej/pliku env | nazwę pliku, trzy wymagane zmienne i niepuste wartości |
| `npx` niedostępne / błąd opcji Node | Node 24, npm i PATH procesu OpenCode |
| Błąd TLS | zaufanie do CA DDEV na hoście; nigdy `NODE_TLS_REJECT_UNAUTHORIZED=0` |
| `404` / błąd zależności | pełną paczkę pluginu, aktywację, AI Abilities i domenę |
| `401` | login, aktywne hasło aplikacyjne, endpoint bez `-oauth`, przekazanie Authorization do PHP |
| `403` przy ability | uprawnienia konta, superadmin multisite i reguły pluginu |
| Niewłaściwa witryna | host w `WP_API_URL`, faktyczny blog ID i URL z odczytu |
| Figma wymaga logowania | autoryzację `figma` w `/mcps`, niezależnie od Novamiry |
| MariaDB wyłączona | lokalne `disabled: false`, update i restart OpenCode |
| MariaDB buduje się długo | postęp stderr, miejsce na dysku, dostęp do rejestrów i emulację amd64 |
| Błąd checksum archiwum | przerwij start; nie pomijaj kontroli SHA-256 |
| Docker niedostępny | działanie Linux Docker Engine i uprawnienia procesu OpenCode |
| MariaDB connection refused | uruchomienie DDEV, prawidłowy port hosta i wybraną sieć |
| MariaDB access denied | konto, hasło, dozwolony host i pełne granty do zamierzonej bazy |

Nie wklejaj sekretów ani pełnych zrzutów środowiska do logów/czatu.
Novamira daje szeroki dostęp do WordPressa; używaj jej na local/dev/staging,
z backupami i zapisami ograniczonymi do zleconego zadania.

Źródła: [Novamira quick start](https://novamira.ai/quickstart/),
[most WordPress 0.4.0](https://github.com/Automattic/mcp-wordpress-remote/tree/7190a4ccf9a16b909433133ecf90c52c9bb04da4),
[OpenCode MCP](https://opencode.ai/v2/docs/mcp-servers),
[Figma MCP](https://developers.figma.com/docs/figma-mcp-server/remote-server-installation/).
