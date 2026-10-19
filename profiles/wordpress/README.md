# Profil WordPress: Novamira + Figma

Jedna instalacja profilu, następnie konfiguracja połączeń dla Twojego projektu.
Nie używamy Pi. Novamira korzysta z loginu i **hasła aplikacyjnego WordPressa**, bez OAuth.
Profil wybiera `novamira` i `figma` w swoim `mcp.json`; ich jedyne definicje
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
- definicje MCP `novamira` i `figma` w zarządzanej konfiguracji OpenCode;
- adapter `.opencode/ai-profile-mcp.py`, który uruchamia most Novamira;
- tę instrukcję w `.opencode/ai-profile/README.md`;
- przykłady konfiguracji w `.opencode/skills/mcp-novamira/examples/`.

**Novamira jest początkowo wyłączona**, aby instalacja działała bez sekretów
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

## 8. Aktualizacje, odinstalowanie i diagnostyka

```bash
make status PROJECT=/sciezka/do/projektu
make update PROJECT=/sciezka/do/projektu
make uninstall PROJECT=/sciezka/do/projektu
```

Te komendy zarządzają wyłącznie profilem i jego plikami. **Nie instalują,
nie aktualizują, nie wyłączają ani nie usuwają pluginu Novamira, danych WordPressa
czy hasła aplikacyjnego**. Zachowują lokalną konfigurację i plik env.
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

Nie wklejaj sekretów ani pełnych zrzutów środowiska do logów/czatu.
Novamira daje szeroki dostęp do WordPressa; używaj jej na local/dev/staging,
z backupami i zapisami ograniczonymi do zleconego zadania.

Źródła: [Novamira quick start](https://novamira.ai/quickstart/),
[most WordPress 0.4.0](https://github.com/Automattic/mcp-wordpress-remote/tree/7190a4ccf9a16b909433133ecf90c52c9bb04da4),
[OpenCode MCP](https://opencode.ai/v2/docs/mcp-servers),
[Figma MCP](https://developers.figma.com/docs/figma-mcp-server/remote-server-installation/).
