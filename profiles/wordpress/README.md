# Profil WordPress: Novamira + Figma + MariaDB + Playwright

Jedna instalacja profilu, następnie konfiguracja połączeń dla Twojego projektu.
Nie używamy Pi. Novamira korzysta z loginu i **hasła aplikacyjnego WordPressa**, bez OAuth.
Profil wybiera `novamira`, `figma`, `mariadb` i `playwright` w swoim `mcp.json`; ich jedyne definicje
znajdują się w `mcp/servers/`. Figma jest współdzielona z profilem React,
ale każdy projekt zachowuje własne lokalne ustawienia i połączenie.

## 1. Zainstaluj profil

Z katalogu repozytorium `ai-profiles`:

```bash
make install CLIENT=opencode PROFILE=wordpress PROJECT=/sciezka/do/projektu
```

Podaj istniejący katalog repozytorium WordPressa. Jeśli projekt ma już profil,
użyj `make update CLIENT={{AI_PROFILE_CLIENT}} PROJECT=...` dla zainstalowanego WordPress albo
`make switch CLIENT={{AI_PROFILE_CLIENT}} PROFILE=wordpress PROJECT=...` dla innego profilu.

Instalacja dostarcza:

- wspólne i wordpressowe skille do `{{AI_PROFILE_ROOT}}/plugin/skills/`, niezależnie dla wskazanego klienta;
- definicje MCP `novamira`, `figma`, `mariadb` i `playwright` w zarządzanej konfiguracji wskazanego klienta;
- adapter Rust `{{AI_PROFILE_ROOT}}/ai-profile-mcp`, który uruchamia most Novamira;
- wspólny bundle `.ai-profiles/mcp/novamira/`: Dockerfile, Compose, launcher,
  `install-plugin`, zweryfikowany ZIP, pusty `.env.novamira` i prywatny katalog;
- launcher Rust `{{AI_PROFILE_MCP}}/mariadb/mariadb-launcher`, który buduje i uruchamia
  oficjalny serwer MCP w Dockerze;
- launcher Rust i kontekst ręcznego buildu Playwright w `{{AI_PROFILE_MCP}}/playwright/`;
- tę instrukcję wraz z przykładami konfiguracji w `{{AI_PROFILE_ROOT}}/README.md`;
- skill SQL w `{{AI_PROFILE_ROOT}}/plugin/skills/mcp-mariadb/`;
- skrypt importu UpdraftPlus i jego pliki pomocnicze w `{{AI_PROFILE_ROOT}}/scripts/`.

W źródłowej instrukcji `{{AI_PROFILE_ROOT}}`, `{{AI_PROFILE_MCP}}`,
`{{AI_PROFILE_LOCAL}}` i `{{AI_PROFILE_CLIENT}}` są znacznikami renderowanymi
podczas instalacji dla wybranego klienta. Dla nowych instalacji root to
`.ai-profiles/<klient>`. Możesz wybrać CLIENT=claude lub CLIENT=codex bez OpenCode.

**Novamira jest domyślnie włączona**, a MariaDB początkowo wyłączona.
Instalacja nie łączy się z WordPressem i działa bez sekretów. Przygotowuje pusty
szablon `.ai-profiles/mcp/novamira/.env.novamira` od razu z prawami `0600`; istniejącego pliku nie zmienia.
Do czasu uzupełnienia danych start MCP zwróci czytelny błąd, bez próby logowania
pustymi wartościami. Figma jest skonfigurowana, ale wymaga autoryzacji.
Pełny ZIP Novamiry 1.12.12 jest dołączony do repozytorium i kopiowany do bundle’a;
instalacja profilu nie pobiera ZIP-a z sieci. Nie instaluje pluginu w WordPressie
ani hostowego Node/npm. Do instalacji pluginu służy osobno uruchamiany skrypt;
aktywacja i AI Abilities pozostają ręczne.
Skille dotyczące Sage/ACF stosuj tylko do pasującej architektury projektu;
zawarte w nich przykłady nie tworzą automatycznie pól, treści ani konfiguracji motywu.

## 2. Przygotuj środowisko

- Rust/Cargo 1.85+, Make i Git w checkoutcie instalatora. Zainstalowany adapter
  Rust nie wymaga Python ani Cargo.
- Wybrany klient (OpenCode, Claude Code albo Codex) dostępny na hoście, na którym pracujesz.
- Lokalny Linux Docker Engine i plugin Compose. Most Node i jego zależności
  są w obrazie; host nie potrzebuje Node, npm ani npx.
- Działający WordPress przez HTTPS. Przy DDEV uruchom projekt przez `ddev start`.
- Dla lokalnego HTTPS: publiczny CA w `.ai-profiles/mcp/novamira/.private/ca.crt`,
  bez symlinków, czytelny dla procesu w kontenerze. Nigdy nie kopiuj klucza CA
  i nie wyłączaj weryfikacji TLS. Publiczne HTTPS używa zaufania obrazu.
- Dostęp do rejestrów Docker/npm podczas ręcznego buildu obrazu. Instalacja
  profilu nie pobiera ZIP-a, nie buduje obrazu i nie uruchamia MCP.

Przypięta paczka to Novamira **1.12.12**, wymagająca WordPress **6.9+** i PHP **8.0+**.
Historyczny audyt
`wordpress-config.md` nie jest konfiguracją tego projektu: nie kopiuj z niego
domen, kont ani ustawień Pi/Codex.

## 3. Zainstaluj plugin Novamira w WordPressie

ZIP jest już w `.ai-profiles/mcp/novamira/downloads/novamira.zip`. Źródło:
[oficjalny release 1.12.12](https://github.com/use-novamira/novamira/releases/tag/v1.12.12),
SHA-256: `ad4dac087e9cad073b2206cb75b44bf5224d57880452f1937a038068920caf78`.
Strona informacyjna producenta to [novamira.ai/download](https://novamira.ai/download/).

W katalogu projektu uruchom **jeden** wariant:

```sh
.ai-profiles/mcp/novamira/install-plugin --wordpress /sciezka/do/wordpressa
# albo dla już uruchomionego lokalnego DDEV:
.ai-profiles/mcp/novamira/install-plugin --ddev
```

Skrypt wymaga WP-CLI (w wariancie DDEV wewnątrz DDEV) i środowiska WordPressa
`local` lub `development`. Odczytuje kontekst WordPressa i respektuje WP_PLUGIN_DIR,
także przy niestandardowym katalogu pluginów. Sprawdza sumę ZIP-a przed dostępem
do WordPressa. Nie nadpisuje istniejącego pluginu, nie zmienia URL, kont ani haseł
i nie uruchamia DDEV. Nie służy do aktualizacji istniejącej wtyczki.

Następnie w panelu **docelowej witryny** ręcznie aktywuj Novamirę i w menu
**Novamira → Configuration** włącz **AI Abilities**. Nie aktywuj sieciowo bez potrzeby.

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

To osobne hasło integracji, **nie Twoje hasło do wp-admin**. Wymagane uprawnienia
to `manage_options` w single-site (zwykle administrator),
a w multisite status superadministratora sieci. Hasło dziedziczy uprawnienia
konta; sama nazwa hasła nie ogranicza dostępu do odczytu.

Jeśli sekcji nie ma, sprawdź HTTPS i ustawienia pluginów bezpieczeństwa,
które mogą wyłączać Application Passwords. Standardowe hasła aplikacyjne nie
mają czasowego wygaśnięcia, ale mogą zostać unieważnione lub zablokowane polityką.

## 5. Zapisz lokalne połączenie

W katalogu **projektu WordPressa**, przed wpisaniem sekretów, samodzielnie zabezpiecz
plik przed przypadkowym commitem, np. dodając do `.gitignore`:

```gitignore
/.ai-profiles/mcp/novamira/.env.novamira
```

Jeśli plik z sekretami był już śledzony przez Git, samo `.gitignore` nie wystarczy.
Usuń go z indeksu i unieważnij ujawnione hasło. Nie zapisuj sekretów w przykładach
wewnątrz `{{AI_PROFILE_ROOT}}/plugin/skills/`: są to zarządzane pliki profilu.

Instalator **nie dodaje** tej reguły ani innego mechanizmu ignorowania.
Szablon jest już przygotowany w `.ai-profiles/mcp/novamira/.env.novamira`.
To plik użytkownika, nie zarządzany payload: update, switch, instalacja kolejnego
klienta i uninstall go nie nadpisują ani nie usuwają. Symlink lub kolizja
z katalogiem/innym typem pliku blokuje przygotowanie profilu.
Sprawdź prawa istniejącego pliku przed wpisaniem sekretów (nowy ma `0600`).
Uzupełnij go lokalnie w edytorze:

```dotenv
WP_API_URL=https://moj-projekt.ddev.site/wp-json/mcp/novamira
WP_API_USERNAME=twoj-login-wordpress
WP_API_PASSWORD="wygenerowane haslo aplikacyjne"
```

Podaj pełny adres z `/wp-json/mcp/novamira`, **nie `/novamira-oauth`**,
oraz login konta, na którym wygenerowano hasło. Cudzysłowy pozwalają zachować spacje.
Plik nie jest skryptem powłoki — nie wpisuj do niego komend ani interpolacji `$VAR`.

Nie musisz tworzyć `{{AI_PROFILE_LOCAL}}` ani aktywować MCP: domyślna definicja
wskazuje `.ai-profiles/mcp/novamira/.env.novamira` i `disabled: false`.
Ścieżki Novamiry są stałe: lokalne nadpisania `env_file`, `command` i `cwd` są
odrzucane. Możesz nadal świadomie wyłączyć MCP lub zmienić timeout w lokalnym
pliku klienta. Inne lokalne pliki env MCP również muszą być pod `.ai-profiles/`.
Nie potrzebujesz ręcznego eksportowania zmiennych do sesji OpenCode.
Upewnij się, że serwer WWW blokuje dostęp HTTP do plików `.env*`, `.ai-profiles/` i `.opencode/`.
Uprawnienia `600` i `.gitignore` nie zastępują konfiguracji serwera WWW.

## 6. Uruchom połączenie w wybranym kliencie

Przebieg: **instalacja profilu → skrypt instalacji pluginu → ręczna aktywacja,
AI Abilities i hasło aplikacyjne → uzupełnienie env → build obrazu → restart klienta/MCP**.

Z katalogu projektu zbuduj obraz bez sekretów w kontekście:

```sh
docker build -t ai-profiles-novamira:0.1.0 .ai-profiles/mcp/novamira
```
Nie ma dodatkowego polecenia aktywacji. Tylko po zmianie lokalnej konfiguracji
(np. jawnego `disabled: true` lub timeoutu) wykonaj standardowe
`make update CLIENT={{AI_PROFILE_CLIENT}} PROJECT=/sciezka/do/projektu`, potem restart.

Zrestartuj wybranego klienta i sprawdź jego listę MCP (dla OpenCode: `opencode mcp list` lub `/mcps`).
Novamira powinna się połączyć bez przeglądarki i bez autoryzacji OAuth:

```text
OpenCode → adapter ai-profiles → most Node w Dockerze → HTTPS + Application Password → Novamira w WordPressie
```

Adapter czyta plik env przy uruchomieniu MCP. Zmienne są przekazywane tylko do
procesu mostu; hasło nie trafia do argumentów komendy, generowanego `opencode.json`
ani stanu instalacji. `OAUTH_ENABLED=false` jest ustawione w definicji profilu.
Nie dodawaj `JWT_TOKEN`, alternatywnych nagłówków uwierzytelniania ani nadpisania
`OAUTH_ENABLED` w lokalnym środowisku mostu.

Poproś agenta o **discovery, odczyt schematu i odczyt ID bloga oraz adresu witryny**.
Potwierdź właściwy cel. Nie trzeba tworzyć strony testowej ani zmieniać ustawień.
Samo otwarcie endpointu w przeglądarce nie jest testem MCP: GET może zwracać `405`.

**Rozbieżność celu blokuje zapis.** Jeśli instrukcje projektu wskazują inną domenę
niż odczytana przez Novamirę albo Novamira nie rozpoznaje wskazanej domeny, agent
ma zgłosić oczekiwany host oraz faktyczny host/blog ID i poprosić użytkownika
o wyjaśnienie. Lista witryn, blog ID `1`, przekierowanie czy podobny tytuł strony
nie upoważniają do wybrania innej witryny. Nie wolno samodzielnie uznać domen
za aliasy ani zmieniać konfiguracji WordPressa lub omijać blokady przez SQL/WP-CLI.
Po wyjaśnieniu trzeba ponownie potwierdzić cel odczytem. Przed każdym zapisem
agent sprawdza też obiekt na tej witrynie; ID treści nie jest globalne między
blogami. Przeniesienie strony do kosza jest operacją destrukcyjną i podlega tym
samym zasadom. Reguły są dystrybuowane w skillach `mcp-novamira` i `acf`;
to instrukcje dla agenta, nie techniczna blokada operacji po stronie MCP.

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
/.ai-profiles/mcp/mariadb/.env.mariadb
```

Utwórz plik i ustaw prawa przed wpisaniem hasła:

```bash
mkdir -p .ai-profiles/mcp/mariadb
(umask 077; touch .ai-profiles/mcp/mariadb/.env.mariadb)
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

Do istniejącego `{{AI_PROFILE_LOCAL}}` dodaj wpis `mariadb` wewnątrz `mcp`:

```json
{
  "mcp": {
    "mariadb": {
      "disabled": false,
      "env_file": ".ai-profiles/mcp/mariadb/.env.mariadb"
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
make update CLIENT={{AI_PROFILE_CLIENT}} PROJECT=/sciezka/do/projektu
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
Serwer WWW musi blokować dostęp HTTP do `.env*`, `.ai-profiles/` i `.opencode/`.

## 9. Import bazy UpdraftPlus do lokalnego DDEV

Profil dostarcza natywne narzędzie Rust **`restore-updraft`**. Pobierasz plik bazy z UpdraftPlus
ręcznie, wskazujesz jego ścieżkę, a narzędzie wykonuje lokalny backup, import
i dostosowanie URL. To **zastąpienie całej bazy `db`**, nie dopisanie rekordów
ani synchronizacja wybranych tabel. Nie używa MCP i nie pobiera nic z produkcji.
Sama instalacja/aktualizacja profilu nigdy nie uruchamia importu.

### Wymagania

- Host Linux/Unix, Docker i DDEV oraz uruchomiony projekt. Python ani Cargo
  nie są wymagane do uruchomienia zainstalowanego narzędzia.
- **Nie instalujesz globalnie WP-CLI.** Korzystamy z WP-CLI w kontenerze `web`:
  `ddev exec -- wp ...`. Standardowy obraz webowy DDEV dostarcza WP-CLI.
- Zainstalowany/zaktualizowany profil WordPress i standardowy WordPress
  z `wp-config.php` oraz `wp-content/` w docroot DDEV.
- Istniejąca, zainicjalizowana lokalna baza WordPressa `db`, na usłudze `db`
  (`DB_HOST=db` lub `db:3306`). Skrypt nie działa z produkcyjnym/remote DB_HOST.
- `WP_ENVIRONMENT_TYPE` ustawione na `local` w lokalnym wp-config lub środowisku.
  Skrypt nie zmienia sam plików konfiguracyjnych WordPressa.
- Pełny, **zaufany i nieszyfrowany** backup bazy `.sql` lub `.gz`. Nie ZIP z plikami,
  nie `.crypt`, nie pojedynczy fragment podzielonej bazy. Prefiks tabel i tryb
  single-site/multisite muszą odpowiadać lokalnej instalacji.
- Każdy docelowy host musi już występować w `ddev describe`, również host BIP.
  Skrypt nie dodaje hostów i nie zmienia topologii/config DDEV.

Ta wersja nie obsługuje pustej bazy startowej, automatycznej zmiany prefiksu,
Bedrock ani niestandardowych ścieżek WP_CONTENT_DIR/WPMU_PLUGIN_DIR.
SQL z backupu jest kodem wykonywanym w lokalnej bazie: kontrola formatu nie jest
sandboxem ani pełnym audytem bezpieczeństwa dumpa. Nie importuj nieznanych plików.

### Najpierw plan — bez importu

W katalogu projektu, po aktualizacji profilu:

```bash
{{AI_PROFILE_ROOT}}/scripts/restore-updraft \
  ~/Downloads/backup-twoja-strona-db.gz \
  --project "$PWD" \
  --map https://twoja-strona.pl=https://moj-wordpress.ddev.site
```

Dla multisite podaj **każdą witrynę**, np. dodatkowo:

```bash
{{AI_PROFILE_ROOT}}/scripts/restore-updraft \
  ~/Downloads/backup-twoja-strona-db.gz \
  --project "$PWD" \
  --map https://twoja-strona.pl=https://moj-wordpress.ddev.site \
  --map https://bip.twoja-strona.pl=https://bip-moj-wordpress.ddev.site
```

Mapowanie obejmuje origin URL (schemat + host + opcjonalny port), bez ścieżek.
Ścieżki witryn są zachowywane. Jeśli źródło używa HTTP i HTTPS, podaj oba warianty
jako osobne `--map`. Jedna domena źródłowa nie może wskazywać różnych hostów,
a różne domeny nie mogą zostać scalone pod jednym hostem. Nie używaj mapowań,
w których nowy URL zawiera stary lub prowadzi do kolejnej zamiany.

Plan sprawdza projekt, połączenie, prefiks/tryb dumpa, integralność gzip i pokazuje
mapowania. **Nie importuje SQL, nie tworzy backupu i nie zapisuje ochrony MU.**
To nie jest symulacja wykonania całego SQL: pełna topologia i kompletność mapowań
home/siteurl są weryfikowane dopiero po imporcie. Brakujące mapowanie przerywa
dalsze zmiany — lokalna baza jest wtedy już zastąpiona, a backup pozostaje dostępny.

### Wykonanie

Do tej samej komendy dopisz `--apply`. Skrypt pokaże cel i poprosi o wpisanie
**nazwy projektu DDEV**. Nie ma flagi pomijającej potwierdzenie.

Następnie:

1. Ponownie sprawdzi źródło/cel i zablokuje równoległy import tym samym skryptem.
2. Skopiuje wejściowy dump do prywatnego katalogu, sprawdzając SHA-256.
3. Wykona `ddev export-db` i zweryfikuje backup przed uruchomieniem `import-db`.
4. Doda lokalną ochronę MU (poniżej) i zastąpi całą lokalną bazę `db`.
5. Odczyta faktyczne witryny i sieci multisite oraz sprawdzi wszystkie mapowania.
6. Dostosuje strukturalne domeny/home/siteurl; resztę URL zmieni przez
   `wp search-replace --precise --all-tables-with-prefix --skip-columns=guid`.
   WP-CLI obsługuje serializowane dane — nie robimy ślepej zamiany w pliku SQL.
7. Sprawdzi zamiany przez kolejne `--dry-run`, zweryfikuje URL/blogi/sieci,
   wyłączy indeksowanie (`blog_public=0`) i sprawdzi `wp core is-installed`.

Nie edytuje `wp-config.php`, stałych WP_HOME/WP_SITEURL/DOMAIN_CURRENT_SITE,
grantów SQL, motywu, uploadów ani kodu pluginu Novamira. Nie resetuje dodatkowo
haseł; konta i hasła WordPressa są jednak zastępowane zawartością dumpa.
Stałe konfiguracyjne i ścieżki sieci muszą pasować do planowanej lokalnej topologii.
GUID-y pozostają bez zmian. Nie ma gwarancji naprawienia każdego cache/pluginowego
formatu danych; przed rozpoczęciem zwykłej pracy sprawdź frontend i panel.

### Ochrona przed lokalną wysyłką

Skrypt tworzy `wp-content/mu-plugins/ai-profile-local-safety.php`. Plik:

- jest aktywny wyłącznie przy `IS_DDEV_PROJECT=true`;
- blokuje `wp_mail`, zaplanowane zadania WP-Cron i zewnętrzne żądania przez
  WordPress HTTP API, w tym webhooki używające tego API;
- pozostaje po imporcie, aby samo otwarcie witryny nie uruchomiło tych integracji.

Nie nadpisujemy pliku o innej zawartości. Dodaj lokalny plik do projektowego
`.gitignore`, jeśli nie chcesz go wersjonować:

```gitignore
/wp-content/mu-plugins/ai-profile-local-safety.php
```

To nie jest firewall: osobne workery, systemowy cron, drop-iny i pluginy używające
bezpośrednio cURL/SMTP mogą ominąć filtry WP. Przed importem zatrzymaj lokalne
workery i nie otwieraj witryny w trakcie operacji. Sprawdź również lokalne
ustawienia Redis/cache i usług zewnętrznych. Skrypt nie wykonuje globalnego
`wp cache flush`, który mógłby naruszyć współdzielony cache.

### Backup i przerwany import

Backup pozostaje poza repozytorium/docroot:
`$XDG_STATE_HOME/ai-profiles/wordpress-backups/<projekt>/restore-*/before-import.sql.gz`
(domyślnie `~/.local/state/ai-profiles/wordpress-backups/`). Katalog pojedynczego
importu ma prawa `700`, plik backupu `600`. Ścieżka jest wypisywana przed importem.
Kopia wejściowego dumpa jest usuwana po zakończeniu/przerwaniu operacji.
Oryginalnego pliku w Downloads nie zmieniamy.

Import nie jest transakcją. Przy błędzie **nie ma automatycznego rollbacku**:
skrypt wypisze konkretną komendę odtworzenia zachowanego backupu, np.:

```bash
cd /sciezka/do/projektu
ddev import-db --database=db --file=/prywatna/sciezka/before-import.sql.gz
```

Blokada chroni tylko przed drugim uruchomieniem tego skryptu, nie przed innym
klientem SQL czy importem. Nie uruchamiaj równolegle innych operacji zapisu.
Przy awarii procesu/systemu kopia wejściowa może pozostać w prywatnym katalogu.
Backupów nie commituj ani nie publikuj — zawierają dane i sekrety witryny.

### Po imporcie

- Sprawdź wszystkie lokalne hosty, panel, media, linki i cache projektu.
- Pliki uploadów, pluginów i motywu nie są odtwarzane przez backup samej bazy.
- W panelu Novamiry ponownie włącz AI Abilities dla lokalnej domeny.
- Import może zastąpić lokalne konta i hasła aplikacyjne. Utwórz nowe lokalne
  Application Password i uzupełnij `.env.novamira`; skrypt nie kopiuje sekretów
  kont ani nie tworzy automatycznie użytkowników.
- Zweryfikuj lokalną ochronę mail/HTTP/cron przed ewentualnym usunięciem MU-pluginu.
- `make uninstall` usuwa skrypt dostarczony z profilem, ale nie usuwa backupów
  ani ochrony MU, którą importer utworzył poza zarządzanymi plikami profilu.

## 10. Aktualizacje, odinstalowanie i diagnostyka

```bash
make status PROJECT=/sciezka/do/projektu
make update CLIENT={{AI_PROFILE_CLIENT}} PROJECT=/sciezka/do/projektu
make uninstall CLIENT={{AI_PROFILE_CLIENT}} PROJECT=/sciezka/do/projektu
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
| Novamira wyłączona | świadome lokalne `disabled: true` / `enabled: false`; zmień tylko jeśli chcesz ją włączyć, potem update i restart |
| Brak zmiennej/pliku env | nazwę pliku, trzy wymagane zmienne i niepuste wartości |
| Most nie startuje | Docker Engine, Compose, ręczny build obrazu i dostęp klienta do demona |
| Błąd TLS | zaufanie do CA DDEV na hoście; nigdy `NODE_TLS_REJECT_UNAUTHORIZED=0` |
| `404` / błąd zależności | pełną paczkę pluginu, aktywację, AI Abilities i domenę |
| `401` | login, aktywne hasło aplikacyjne, endpoint bez `-oauth`, przekazanie Authorization do PHP |
| `403` przy ability | uprawnienia konta, superadmin multisite i reguły pluginu |
| Błąd inicjalizacji Novamiry | panel MCP klienta pokazuje bezpieczny błąd; HTTP 401 oznacza potwierdzone odrzucenie danych, pozostałe awarie dostają komunikat ogólny |
| Niewłaściwa witryna | host w `WP_API_URL`, faktyczny blog ID i URL z odczytu |
| Figma wymaga logowania | autoryzację `figma` w `/mcps`, niezależnie od Novamiry |
| MariaDB wyłączona | lokalne `disabled: false`, update i restart OpenCode |
| MariaDB buduje się długo | postęp stderr, miejsce na dysku, dostęp do rejestrów i emulację amd64 |
| Błąd checksum archiwum | przerwij start; nie pomijaj kontroli SHA-256 |
| Docker niedostępny | działanie Linux Docker Engine i uprawnienia procesu OpenCode |
| MariaDB connection refused | uruchomienie DDEV, prawidłowy port hosta i wybraną sieć |
| MariaDB access denied | konto, hasło, dozwolony host i pełne granty do zamierzonej bazy |

Nie wklejaj sekretów ani pełnych zrzutów środowiska do logów/czatu.
Adapter nie uznaje `connectionFailed` mostu (również `{}`) za udane połączenie.
Nie diagnozuje hasła na podstawie braku narzędzi. Nie przekazuje surowych logów
mostu ani odpowiedzi błędów inicjalizacji, które mogą zawierać sekrety.
Novamira daje szeroki dostęp do WordPressa; używaj jej na local/dev/staging,
z backupami i zapisami ograniczonymi do zleconego zadania.

## Audyt WCAG przez Playwright

Profil udostępnia serwer MCP `playwright` i skill `wcag-audit`. Instalacja z kroku 1
dostarcza launcher, Dockerfile i Compose do `{{AI_PROFILE_MCP}}/playwright/`.
Z katalogu głównego docelowego projektu wykonaj:

```sh
docker build --tag ai-profiles-playwright:0.2.0 {{AI_PROFILE_MCP}}/playwright
```

Do użycia potrzebny jest Docker Engine i plugin Compose, bez hostowego
Node/npm/Chromium na potrzeby Playwright (Novamira ma osobne wymagania).
Nie ma automatycznego buildu/pull ani publikacji GHCR.
Opcjonalnie zapisz lokalnym edytorem `.ai-profiles/.private/playwright-basic-auth.json`
o prawach `0600` lub bardziej restrykcyjnych, bez symlinków:

```json
{
  "username": "LOGIN",
  "password": "HASLO",
  "origin": "https://serwis.example:8443"
}
```

Wszystkie trzy pola są wymagane; origin to schemat/host/port bez ścieżki i ogranicza
wysyłanie credentials do tego serwisu. Instalator dopisuje regułę
`/.ai-profiles/.private/playwright-basic-auth.json` do `.gitignore`, ale nie odśledza już
wersjonowanych plików. Nie podawaj sekretów agentowi ani nie zapisuj ich w raportach.
Brak pliku oznacza brak Basic Auth. Zrestartuj klienta i zatwierdź jego natywne
zgody projektu/MCP, jeśli wymagane.

Launcher automatycznie tworzy `.ai-profiles/reports/`, dołącza opcjonalny Basic Auth i dostępny
publiczny CA mkcert oraz używa gotowej sieci hosta Linux; nie konfiguruje VPN ani
hosta. Docker build nie tworzy katalogu raportów. Zwykłe użycie nie wymaga edycji
Compose ani `{{AI_PROFILE_LOCAL}}`.
Szczegóły znajdują się w dostarczonej instrukcji `{{AI_PROFILE_MCP}}/playwright/README.md`.
Artefakty audytu trafiają do `.ai-profiles/reports/<timestamp>/`; auth storage pozostaje
poza raportami. Audyt nie zmienia kodu aplikacji i nie powinien wysyłać
formularzy tworzących trwałe dane bez zgody.

Wyjątki: niestandardowy CA wskaż przez `AI_PROFILE_PLAYWRIGHT_CA_CERT` w
`{{AI_PROFILE_LOCAL}}`, nigdy przez wyłączenie TLS. Przy braku obrazu wykonaj build;
przy braku uprawnień sprawdź dostęp klienta do Dockera/projektu. HTTP 401 wymaga
lokalnego sprawdzenia loginu, hasła, origin (także portu) i praw pliku oraz restartu MCP.
Jawne nazwy screenshotów to `.ai-profiles/reports/...`; `screenshot.png` w korzeniu RO nie działa.

WordPress **3.0.0** wprowadza nowy układ bez automatycznej migracji i fallbacku
do starych lokalizacji: Novamira w `.ai-profiles/mcp/novamira/`, prywatne dane
Playwright w `.ai-profiles/.private/`, raporty w `.ai-profiles/reports/`.
Nie przenosimy ani nie usuwamy starych poświadczeń. Nowe instalacje wykonują
opisany wyżej flow. Wspólny bundle Novamiry jest zachowany po uninstall; różniące
się receptury lub ZIP blokują nadpisanie. Ręczne zmiany zarządzanych wpisów
klienta nadal blokują update. Dla Novamiry nie powstaje reguła `.gitignore`:
samodzielnie zabezpiecz sekrety przed commitem i udostępnieniem przez HTTP.
Pełna instrukcja bundle: `.ai-profiles/mcp/novamira/README.md`.

Źródła: [Novamira quick start](https://novamira.ai/quickstart/),
[most WordPress 0.4.0](https://github.com/Automattic/mcp-wordpress-remote/tree/7190a4ccf9a16b909433133ecf90c52c9bb04da4),
[OpenCode MCP](https://opencode.ai/v2/docs/mcp-servers),
[Figma MCP](https://developers.figma.com/docs/figma-mcp-server/remote-server-installation/).
