# Novamira MCP — audyt i instrukcja odtworzenia konfiguracji

Stan zbadany: 19 października 2026 r. Projekt źródłowy: `/home/kzielinski/projects/wfosigw/wfosigw`.

Dokument jest instrukcją dla kolejnego agenta konfigurującego analogiczne połączenie w innym projekcie WordPress. Rozróżnia konfigurację faktycznie znalezioną, stan potwierdzony przez WP-CLI oraz szablony wymagające dostosowania. Sekrety celowo pominięto: w nowym projekcie należy utworzyć własne hasła aplikacyjne i autoryzacje OAuth.

## 1. Najważniejsze ustalenia

W projekcie istnieją **dwa niezależne sposoby połączenia z tym samym pluginem**:

| Klient | Transport od klienta | Endpoint WordPressa | Uwierzytelnianie |
| --- | --- | --- | --- |
| Codex | Bezpośrednie HTTP MCP | `/wp-json/mcp/novamira-oauth` | OAuth realizowany przez Novamira |
| Pi z `pi-mcp-adapter` | stdio do lokalnego procesu Node, następnie HTTP do WP | `/wp-json/mcp/novamira` | WordPress Application Password, OAuth wyłączony w moście |

Nie zamieniaj tych endpointów. `AGENTS.md` opisuje głównie endpoint `/novamira`; rzeczywista globalna konfiguracja Codex używa `/novamira-oauth`.

Novamira zawiera własny adapter MCP i zależności OAuth. Nie znaleziono potrzeby instalowania osobnego pluginu WordPress MCP Adapter ani zmian w motywie Sage do uruchomienia połączenia.

Samo sklonowanie repozytorium nie odtwarza połączenia. Plugin Novamira, lokalny nginx, dane bazy oraz globalne ustawienia klientów znajdują się poza zestawem wersjonowanych plików projektu.

## 2. Faktycznie potwierdzony stan środowiska

### Wersje

| Komponent | Stan |
| --- | --- |
| WordPress | `7.1`, potwierdzony w runtime i `wp-includes/version.php` |
| Novamira | `1.12.3`, potwierdzony w runtime i kodzie pluginu |
| Minimalny WordPress wymagany przez tę Novamirę | `6.9`, ze względu na Abilities API |
| Minimalne PHP według metadanych pluginu | `8.0` |
| PHP DDEV | konfiguracja `8.4`, runtime `8.4.24` |
| MariaDB DDEV | konfiguracja `11.8` |
| Serwer WWW | `nginx-fpm` |
| Node na hoście podczas audytu | `v24.21.0` |
| Codex CLI na hoście podczas audytu | `0.163.0` |
| Zainstalowany `pi-mcp-adapter` | `5.3.0`; deklaracja zależności `^5.3.0`, wymagany Node `>=20` |
| Most WP dla Pi | skonfigurowane `@automattic/mcp-wordpress-remote@0.4.0` |

Odtwarzanie 1:1 wymaga zachowania wersji pluginu i mostu. Pobranie najnowszego wydania może zmienić nazwy narzędzi, ekrany konfiguracji i zachowanie OAuth. Zapis wersji mostu potwierdzono w konfiguracji; jego połączenia nie uruchamiano w ramach audytu.

### Multisite

| Właściwość | Główna witryna | BIP |
| --- | --- | --- |
| Blog ID | `1` | `2` |
| `home` i `siteurl` | `https://wfosigw.ddev.site` | `https://bip-wfosigw.ddev.site` |
| Domena | `wfosigw.ddev.site` | `bip-wfosigw.ddev.site` |
| Ścieżka bloga | `/` | `/` |
| Novamira w `active_plugins` | tak | tak |
| `novamira_ai_abilities_enabled` | string `"1"` | string `"1"` |
| `novamira_ai_abilities_domain` | `wfosigw.ddev.site` | `bip-wfosigw.ddev.site` |
| Wynik `novamira_is_enabled()` | `true` | `true` |
| Application Passwords dostępne | `true` | `true` |
| `novamira_oauth_schema_version` | `"4"` | `"4"` |
| `novamira_ability_rules` | `false` z `get_option()` | `false` z `get_option()` |
| `novamira_feature_preferences` | `false` z `get_option()` | `false` z `get_option()` |
| `novamira_instructions_enabled` | `false` z `get_option()` | `false` z `get_option()` |
| `novamira_instructions_content` | `false` z `get_option()` | `false` z `get_option()` |
| `novamira_active_design` | `wfosigw-editorial-institutional` | `false` z `get_option()` |

`get_option()` zwracające `false` nie rozróżnia tutaj braku opcji od wartości `false`. Nie traktuj braku zapisanych preferencji funkcji jako wyłączenia wszystkich funkcji: manifest Novamira definiuje wartości domyślne.

**Novamira nie jest aktywowana sieciowo.** Jest aktywowana osobno na obu blogach. W `active_sitewide_plugins` potwierdzono wyłącznie `updraftplus/updraftplus.php`.

Na głównej witrynie aktywne były również ACF Pro, Capability Manager Enhanced, Contact Form 7, PublishPress Permissions (`press-permit-core`), Ultimate Addons for Gutenberg oraz WP Mail SMTP. BIP miał aktywne lokalnie ACF Pro i Novamira. Zestawy te wpływają na dostępne API i abilities, ale nie wszystkie te pluginy są zależnościami transportu MCP.

Domyślny cel pracy agentów to blog `1`. Blog `2` wybiera się na wyraźne polecenie dotyczące BIP. Podczas tego audytu oba blogi odczytano, ponieważ przedmiotem była cała konfiguracja MCP.

## 3. Mapa plików i stanów, które należy odtworzyć

| Lokalizacja | Znaczenie | W Git tego projektu |
| --- | --- | --- |
| `.ddev/config.yaml` | DDEV, PHP, baza, hosty | tak |
| `.ddev/nginx_full/nginx-site.conf` | lokalne reguły nginx, w tym dostęp do OAuth discovery | ignorowana |
| `.ddev/php/php.ini` | limity PHP | tak |
| `wp-config.php` | bootstrap, multisite, cookies | tak |
| `wp-config-local.php` | lokalna baza, domena, środowisko i sekrety WP | ignorowany |
| `wp-config-local.php.example` | szablon lokalnej konfiguracji | tak |
| `wp-content/plugins/novamira/` | plugin wraz z bundled `vendor/` | ignorowany |
| `wp-content/novamira-sandbox/` | pliki runtime Novamira | ignorowany |
| `~/.codex/config.toml` | globalna konfiguracja Codex MCP | poza repozytorium |
| `~/.pi/agent/settings.json` | rozszerzenia Pi | poza repozytorium |
| `~/.pi/agent/mcp-adapter.json` | znalezione połączenie Pi do BIP | poza repozytorium |
| `.pi/mcp-adapter.json` | możliwy lokalny override Pi | **nie istnieje w chwili audytu** |
| `.pi/mcp-secrets.local.json` | lokalne dane logowania WP | ignorowany, istnieje |
| `~/.config/pi/wfosigw-mcp-secrets.json` | dodatkowy plik z sekcjami `editor`, `reader` | poza repozytorium, istnieje |
| `.pi/skills/mcp-novamira/SKILL.md` | zasady pracy agentów przez Novamira | tak |
| `AGENTS.md` | wybór bloga, discovery, schematy i zasady zmian | tak |
| Baza WordPressa | aktywacja, enable/domain, uprawnienia, OAuth, design, skills | poza repozytorium |

Istnienie pliku z sekretami **nie oznacza**, że jest automatycznie używany. Nie znaleziono aktywnego lokalnego konfiguratora wczytującego `.pi/mcp-secrets.local.json` ani pliku z sekcjami `editor` i `reader`.

### Niespójność historycznych konfiguracji Pi

Globalny `~/.pi/agent/mcp-adapter.json` zawiera tylko serwer `bip_wfosigw_local`. Nie znaleziono w sprawdzonych standardowych plikach aktywnej definicji `wfosigw_local` dla Pi.

Plik `~/.pi/agent/mcp-project-approvals.json` zawiera historyczne zgody dla `wfosigw_local`, `wfosigw_editor_local` i `wfosigw_reader_local`. To rejestr wcześniejszych zgód, nie definicje połączeń. Nie dowodzi, że te serwery nadal są skonfigurowane ani że zapewniają obecnie różne poziomy uprawnień.

Nie kopiuj approval hashy, cache, identyfikatorów urządzeń i plików onboarding jako sposobu instalacji. Nowy projekt powinien przejść własne wykrywanie i zatwierdzenie konfiguracji.

## 4. Architektura i endpointy

```text
Codex
  -> HTTP MCP + OAuth Bearer
  -> https://HOST/wp-json/mcp/novamira-oauth
  -> Novamira -> bundled MCP Adapter -> WordPress Abilities API
  -> permission_callback -> API WordPressa / pluginów

Pi + pi-mcp-adapter
  -> stdio
  -> npx -y @automattic/mcp-wordpress-remote@0.4.0
  -> HTTPS + Application Password
  -> https://HOST/wp-json/mcp/novamira
  -> ten sam plugin i zestaw abilities
```

Kod rejestruje również alias `/wp-json/mcp/mcp-adapter-default-server` dla starszych klientów. Nowa konfiguracja powinna używać odpowiedniego endpointu z tabeli w rozdziale 1.

Transport w bundled `includes/Transport/HttpTransport.php` obsługuje POST i sesje MCP; DELETE służy zakończeniu sesji. Kod zaznacza, że streaming SSE przez GET nie jest zaimplementowany i GET zwraca `405`. Samo otwarcie endpointu w przeglądarce nie potwierdza awarii MCP.

W runtime głównej witryny potwierdzono rejestrację wszystkich trzech ścieżek MCP oraz REST OAuth: `/novamira/v1/oauth/register`, `/token`, `/device`, `/revoke`, `/introspect`.

### Trzy narzędzia wejściowe MCP

Nazwy narzędzi publikowane przez Novamira:

1. `mcp-adapter-discover-abilities` — odkrywanie możliwości i instrukcji witryny.
2. `mcp-adapter-get-ability-info` — odczyt schematu konkretnej ability.
3. `mcp-adapter-execute-ability` — uruchomienie wskazanej ability.

Ich wewnętrzne nazwy WordPress to odpowiednio `novamira-mcp-adapter/discover-abilities`, `novamira-mcp-adapter/get-ability-info` i `novamira-mcp-adapter/execute-ability`.

Klient może dodać prefiks serwera i zastąpić myślniki podkreśleniami. Dlatego nazwy widoczne agentowi mogą różnić się od nazw protokołu. Odczytaj listę narzędzi zamiast zakładać prefiks na podstawie `AGENTS.md`.

## 5. Przygotowanie nowego projektu

Przed zmianami ustal: katalog WordPressa, domenę lokalną, czy instalacja jest multisite, blog domyślny, dodatkowe hosty, konto operatora i docelowy klient MCP. Poniższe przykłady używają projektu `nowyprojekt` i opcjonalnego hosta `bip-nowyprojekt`.

### DDEV

Odpowiednik istniejącej konfiguracji:

```yaml
name: nowyprojekt
type: wordpress
docroot: .
composer_root: .
php_version: "8.4"
webserver_type: nginx-fpm
xdebug_enabled: false
additional_hostnames:
  - bip-nowyprojekt
additional_fqdns: []
database:
  type: mariadb
  version: "11.8"
use_dns_when_possible: true
composer_version: "2"
web_environment: []
corepack_enable: false
disable_settings_management: true
```

WFOŚiGW ma dodatkowo `host_db_port: 62335`; to lokalny port bazy, nie warunek działania MCP. Nie rezerwuj tego samego portu dla drugiego uruchomionego projektu.

`disable_settings_management: true` oznacza, że lokalną konfigurację WP trzeba zapewnić samodzielnie. W badanym `wp-config.php` wartości domyślne bazy są placeholderami; nie polegaj na komentarzu w `.example`, że kontener automatycznie uzupełni bazę bez dodatkowej konfiguracji.

Nie przenoś dodatkowego hosta ani ustawień multisite do zwykłej instalacji single-site tylko z powodu MCP. Służą odtworzeniu topologii WFOŚiGW.

### Lokalna konfiguracja WordPressa

Badany `wp-config.php` ładuje `wp-config-local.php` **przed** wartościami domyślnymi. W nowym projekcie zachowaj tę kolejność.

Przykład części lokalnego pliku dla DDEV; to szablon, nie komplet sekretów:

```php
<?php
define('DB_NAME', 'db');
define('DB_USER', 'db');
define('DB_PASSWORD', 'db');
define('DB_HOST', 'db');

define('WP_HOME', 'https://nowyprojekt.ddev.site');
define('WP_SITEURL', 'https://nowyprojekt.ddev.site');
define('WP_ENVIRONMENT_TYPE', 'local');
define('DOMAIN_CURRENT_SITE', 'nowyprojekt.ddev.site');
define('WP_DEBUG', true);
define('WP_DEBUG_LOG', true);
define('WP_DEBUG_DISPLAY', false);
```

Wygeneruj własne klucze i salts WordPressa. Wartości DB w przykładzie są typowymi danymi DDEV; zweryfikuj je dla nowego kontenera. `WP_DEBUG_DISPLAY=false` jest zaleceniem dla nowego projektu, aby komunikaty PHP nie uszkadzały odpowiedzi JSON, a nie ustaleniem wartości tej stałej w pliku źródłowym.

Topologia multisite WFOŚiGW ma:

```php
define('WP_ALLOW_MULTISITE', true);
define('MULTISITE', true);
define('SUBDOMAIN_INSTALL', true);
define('PATH_CURRENT_SITE', '/');
define('SITE_ID_CURRENT_SITE', 1);
define('BLOG_ID_CURRENT_SITE', 1);
define('COOKIE_DOMAIN', '');
```

`COOKIE_DOMAIN=''` jest istotne: `bip-wfosigw.ddev.site` to sąsiedni host, a nie subdomena `wfosigw.ddev.site`. Cookie przypisane do `.wfosigw.ddev.site` nie działałoby na BIP. Ma to znaczenie podczas logowania do wp-admin i zgody OAuth.

Przy odtworzeniu bazy z innego środowiska sprawdź domeny sieci i obu blogów oraz ich `home`/`siteurl`. Nie wykonuj globalnego search-replace domen bez wcześniejszego rozpoznania multisite.

## 6. Nginx i certyfikaty — krytyczne dla OAuth

W tym projekcie istnieje lokalny override `.ddev/nginx_full/nginx-site.conf`, którego Git nie przenosi. Kluczowa reguła dopuszcza `/.well-known/`, zachowując blokadę pozostałych ukrytych ścieżek:

```nginx
location ~* /\.(?!well-known\/) {
    deny all;
}

location / {
    absolute_redirect off;
    try_files $uri $uri/ /index.php?$args;
}
```

W sekcji PHP istnieją m.in.:

```nginx
fastcgi_param SERVER_NAME $host;
fastcgi_param HTTPS $fcgi_https;
fastcgi_intercept_errors off;
fastcgi_read_timeout 10m;
```

Pełny plik z audytu jest w załączniku na końcu dokumentu. Dla odtworzenia 1:1 przenieś cały override, dostosowując `root` do docroot nowego projektu. Przy istniejącym własnym nginx scal reguły z jego konfiguracją. Po zmianie konfiguracji DDEV wykonaj `ddev restart`.

OAuth discovery tej wersji Novamira:

- `https://HOST/.well-known/oauth-protected-resource`
- `https://HOST/.well-known/oauth-authorization-server`
- `https://HOST/.well-known/openid-configuration`

Protected-resource metadata wskazują zasób `/wp-json/mcp/novamira-oauth`. Authorization server metadata wskazują `issuer=home_url()`, ekran autoryzacji `wp-admin/admin.php?page=novamira-oauth-authorize` i REST token endpoint `/wp-json/novamira/v1/oauth/token`.

Nginx musi kierować te żądania do WordPressa. Reguła bez wyjątku dla `.well-known` spowoduje `403` przed uruchomieniem pluginu.

Certyfikat DDEV musi być zaufany na hoście wykonującym klienta MCP. Pi ma `NODE_OPTIONS=--use-system-ca`, dzięki czemu jego proces Node korzysta z systemowego zaufania CA. Zachowaj tę opcję przy użyciu odpowiadającej jej wersji Node. Nie zastępuj jej `NODE_TLS_REJECT_UNAUTHORIZED=0` ani `curl -k`.

## 7. Instalacja i aktywacja Novamira

1. Zapewnij działający WordPress i lokalną bazę.
2. Przygotuj pełną paczkę Novamira `1.12.3`, najlepiej archiwum z tej instalacji lub oficjalne wydanie o tej wersji. Adres producenta: <https://novamira.ai/download/>.
3. Umieść plugin w `wp-content/plugins/novamira/`.
4. Potwierdź obecność `vendor/autoload_packages.php` i `vendor/novamira/mcp-adapter/autoload.php`.
5. Aktywuj plugin na docelowym blogu. Dla topologii WFOŚiGW powtórz aktywację osobno na drugim blogu.
6. Zaloguj się do panelu jako uprawniony operator i otwórz Novamira → Configuration.
7. Włącz AI Abilities na każdej docelowej domenie.

Nie używaj automatycznego GitHub „Source code ZIP”: kod pluginu jawnie ostrzega, że brak bundled vendor uniemożliwi rejestrację MCP.

Przykłady aktywacji w **nowym** projekcie:

```bash
ddev exec wp --url=https://nowyprojekt.ddev.site plugin activate novamira
ddev exec wp --url=https://bip-nowyprojekt.ddev.site plugin activate novamira
```

Drugie polecenie wykonuj tylko przy faktycznie istniejącym dodatkowym blogu. Nie dodawaj `--network`, jeśli odtwarzasz sposób aktywacji z WFOŚiGW.

### Blokada domeny

`novamira_is_enabled()` wymaga jednocześnie:

```text
novamira_ai_abilities_enabled jest stringiem "1" albo bool true
ORAZ
novamira_ai_abilities_domain === host z home_url()
```

Po przywróceniu backupu z testu sam stan „enabled” może pozostać zapisany, ale domena nie będzie pasować. Ustaw właściwe lokalne URL i ponownie włącz AI Abilities w panelu nowej domeny. Funkcja pluginu `novamira_enable_ai_abilities()` zapisuje obie wartości; preferuj panel zamiast ręcznych zmian SQL.

### Uprawnienia

W tej wersji Novamira `novamira_permission_callback()` wymaga włączenia pluginu i `novamira_current_user_can_manage()`:

- single-site: `manage_options`;
- multisite: `is_super_admin()`.

W audytowanej instalacji konto `m.brzana` istnieje i jest superadministratorem. Ma hasło aplikacyjne o nazwie `Novamira: Pi local`. Odczytano wyłącznie nazwę, bez hasha i hasła.

W nowym projekcie użyj własnego konta. Sama rola administratora pojedynczego bloga w multisite może nie wystarczyć. Nazwy serwerów `editor` lub `reader` nie nadają takich ograniczeń po stronie Novamira.

## 8. Codex — rzeczywista konfiguracja OAuth i odtworzenie

W `~/.codex/config.toml` znaleziono:

```toml
[mcp_servers.wfosigw-local]
url = "https://wfosigw.ddev.site/wp-json/mcp/novamira-oauth"

[mcp_servers.wfosigw-local.tools.mcp-adapter-execute-ability]
approval_mode = "approve"

[mcp_servers.bip-wfosigw-local]
url = "https://bip-wfosigw.ddev.site/wp-json/mcp/novamira-oauth"
```

Zachowano literalną wartość polityki `approve`. Jest to polityka klienta, a nie uprawnienie WordPressa ani zapis zgody OAuth. Przed jej przeniesieniem uwzględnij docelowe zasady zatwierdzania operacji przez agenta.

Szablon dla nowego projektu:

```toml
[mcp_servers.nowyprojekt-local]
url = "https://nowyprojekt.ddev.site/wp-json/mcp/novamira-oauth"

[mcp_servers.nowyprojekt-local.tools.mcp-adapter-execute-ability]
approval_mode = "approve"

[mcp_servers.bip-nowyprojekt-local]
url = "https://bip-nowyprojekt.ddev.site/wp-json/mcp/novamira-oauth"
```

Dopisz odpowiednie sekcje do istniejącego pliku; nie zastępuj całej konfiguracji użytkownika. Sekcję BIP pomiń dla single-site. Alternatywnie oficjalna dokumentacja dopuszcza projektowy `.codex/config.toml` w zaufanym projekcie, ale w WFOŚiGW połączenia znaleziono w konfiguracji globalnej.

Następnie wykonaj lokalnie:

```bash
codex mcp list
codex mcp login nowyprojekt-local
codex mcp login bip-nowyprojekt-local
```

Przejdź logowanie i zgodę na właściwym hoście WordPressa. Autoryzację drugiego hosta wykonuj tylko, jeśli go konfigurujesz. Przeładuj klienta/sesję MCP, aby pojawiły się narzędzia.

Plik TOML nie zawiera tu nagłówka Basic, Application Password ani statycznego Bearer tokenu. Zgody i tokeny OAuth są osobnym stanem klienta i serwera. Nie kopiuj magazynu tokenów ze starego projektu.

Dokumentacja potwierdzająca konfigurację TOML i `codex mcp login`: [OpenAI — MCP w Codex](https://learn.chatgpt.com/docs/extend/mcp?surface=cli). Lokalny stan TOML jest źródłem dla powyższego zrzutu konfiguracji.

### Stan OAuth po stronie WordPressa

Novamira generuje i przechowuje własne klucze w opcjach `novamira_oauth_private_key` i `novamira_oauth_encryption_key`. Nie eksportuj ich do dokumentacji ani konfiguracji klienta.

Schemat OAuth `4` definiuje tabele z prefiksem danego bloga:

```text
<prefix>novamira_oauth_clients
<prefix>novamira_oauth_pending_authorizations
<prefix>novamira_oauth_auth_codes
<prefix>novamira_oauth_access_tokens
<prefix>novamira_oauth_device_codes
<prefix>novamira_oauth_refresh_tokens
```

Plugin instaluje schemat automatycznie. Nie twórz tych tabel ręcznie. Dla nowego projektu wygeneruj nowe klucze i zgody przez normalny przepływ pluginu. Według kodu authorization code ma TTL 1 minuty, access token 1 godziny, refresh token 14 dni; obsługiwane są authorization code + PKCE S256, refresh token oraz device flow. Wydawany scope `mcp` oznacza pełny grant tej integracji, nie profil tylko do odczytu.

## 9. Pi — rzeczywista konfiguracja Application Password i odtworzenie

### Ustawienia klienta

Globalny `~/.pi/agent/settings.json` zawiera pakiet `npm:pi-mcp-adapter` oraz:

```json
{
  "packages": ["npm:pi-mcp-adapter"],
  "extensions": ["-builtin:mcp"]
}
```

To fragment do scalenia z istniejącymi ustawieniami. W rzeczywistym pliku są też inne rozszerzenia, które nie są wymagane przez Novamira. `-builtin:mcp` wyłącza wbudowaną obsługę MCP Pi, aby połączeniem zarządzał zainstalowany adapter.

Wersja adaptera potwierdzona lokalnie to `5.3.0`. Odtworzenie tej wersji:

```bash
pi install npm:pi-mcp-adapter@5.3.0
```

Po instalacji uruchom Pi ponownie. Projektowe `.pi/settings.json` ustawia provider/model i opcje UI, ale nie zawiera połączenia Novamira; wybór modelu nie jest zależnością MCP.

### Znaleziony serwer BIP

W `~/.pi/agent/mcp-adapter.json` znajduje się następująca definicja, z usuniętym sekretem:

```json
{
  "mcpServers": {
    "bip_wfosigw_local": {
      "command": "npx",
      "args": ["-y", "@automattic/mcp-wordpress-remote@0.4.0"],
      "env": {
        "WP_API_URL": "https://bip-wfosigw.ddev.site/wp-json/mcp/novamira",
        "WP_API_USERNAME": "m.brzana",
        "WP_API_PASSWORD": "<APPLICATION_PASSWORD>",
        "NODE_OPTIONS": "--use-system-ca",
        "OAUTH_ENABLED": "false"
      },
      "directTools": true
    }
  }
}
```

`OAUTH_ENABLED` jest stringiem `"false"` w środowisku procesu. `directTools=true` dotyczy ekspozycji narzędzi w Pi, nie uprawnień WordPressa.

### Szablon nowego projektu

Utwórz nowe hasło aplikacyjne w profilu uprawnionego użytkownika WordPressa, nazwij np. `Novamira: Pi local` i zapisz je prywatnie. Nie używaj zwykłego hasła logowania do wp-admin.

Wybierz prywatny plik globalny `~/.pi/agent/mcp-adapter.json` albo ignorowany projektowy `.pi/mcp-adapter.json`. Dla wariantu projektowego:

```json
{
  "mcpServers": {
    "nowyprojekt_local": {
      "command": "npx",
      "args": ["-y", "@automattic/mcp-wordpress-remote@0.4.0"],
      "env": {
        "WP_API_URL": "https://nowyprojekt.ddev.site/wp-json/mcp/novamira",
        "WP_API_USERNAME": "<NOWY_UZYTKOWNIK>",
        "WP_API_PASSWORD": "<NOWE_HASLO_APLIKACYJNE>",
        "NODE_OPTIONS": "--use-system-ca",
        "OAUTH_ENABLED": "false"
      },
      "directTools": true
    }
  }
}
```

Ten szablon odtwarza mechanizm znalezionego połączenia BIP dla witryny głównej nowego projektu. Nie jest zrzutem nieodnalezionej konfiguracji `wfosigw_local`.

Dla drugiego bloga dodaj drugą definicję z inną nazwą serwera i URL. Nie zakładaj współdzielenia opcji między blogami. Konto użytkownika jest sieciowe, ale docelowy blog wybiera host endpointu.

Ustaw prawa `600` na pliku zawierającym sekret. Taki tryb potwierdzono dla znalezionych plików z sekretami i globalnej konfiguracji mostu.

Wariant bez hasła wpisanego do JSON może używać interpolacji `${NOWYPROJEKT_WP_PASSWORD}` w `env`, wspieranej przez kod adaptera 5.3.0. Wtedy zmienna musi być faktycznie wyeksportowana do procesu uruchamiającego Pi. Samo utworzenie pliku JSON z hasłem lub `.env` nie zapewnia jej wczytania. To alternatywa wdrożeniowa, a nie mechanizm potwierdzony w znalezionej definicji BIP.

Po zmianie użyj `/reload` i panelu `/mcp-adapter` w Pi. Projektowa definicja może wymagać zatwierdzenia jako konfiguracja nowego projektu. Nie kopiuj zgód z poprzedniego katalogu.

Dokumentacja adaptera użyta w audycie jest dostępna lokalnie w `~/.pi/agent/npm/node_modules/pi-mcp-adapter/docs/configuration.md` i `docs/servers.md`; [repozytorium autora](https://github.com/nicobailon/pi-mcp-adapter) służy jako punkt odniesienia. Nazwy i zachowanie opisano według lokalnej wersji 5.3.0.

## 10. Dostępne abilities i stan dodatkowego kontekstu

Na głównej witrynie runtime potwierdził m.in.:

- wykonanie PHP: `novamira/execute-php`;
- pliki: `read-file`, `write-file`, `edit-file`, `delete-file`, `disable-file`, `enable-file`, `list-directory` w namespace `novamira/`;
- linki: `novamira/create-upload-link`, `novamira/create-admin-access-link`;
- kontekst: `novamira/agent-context`;
- WP-CLI: `novamira/run-wp-cli`, `novamira/get-wp-cli-job`;
- Gutenberg: `novamira/gutenberg-*`, w tym odczyt i zapis treści oraz kolejka finalizacji bloków;
- skills: `novamira/skill-get`, `skill-write`, `skill-edit`, `skill-delete` oraz prompty skills;
- design: `novamira/list-design-library`, `get-active-design`, `activate-design`, `save-design`, `check-design`, `get-design`, `delete-design`;
- abilities WordPress core i `wp-mail-smtp/get-debug-events`.

Nie wszystkie abilities WordPressa muszą być eksponowane jako osobne narzędzia MCP. Wzorzec tego projektu korzysta z trzech narzędzi wejściowych opisanych wcześniej.

Dostępność WP-CLI zależy od obecności polecenia `wp` oraz funkcji PHP `exec` i `proc_open`. Runtime potwierdził rejestrację obu abilities WP-CLI na blogu 1. Zestaw abilities trzeba odkryć ponownie w projekcie docelowym.

Preferencje funkcji zapisuje opcja `novamira_feature_preferences`, reguły wyłączenia poszczególnych abilities `novamira_ability_rules`. Manifest zawiera domyślnie aktywne moduły infrastruktury, kontekstu użytkownika, design, chat, visual i Block Editor Queue. Brak zapisanych preferencji w obu blogach oznacza konieczność odczytania tych domyślnych reguł, a nie ich zastępowania pustą listą aktywnych modułów.

Aktywny design `wfosigw-editorial-institutional` na blogu 1 to dodatkowa zawartość projektu. Jego treści nie eksportowano w tym audycie. Dla nowego projektu należy utworzyć własny design. Jeżeli wymagane jest także identyczne zachowanie kontekstu projektowego WFOŚiGW, przenieś odpowiedni wpis CPT `novamira_design`, metadane i powiązania, a następnie aktywuj go. Samo ustawienie sluga w opcji nie odtwarza treści designu. Własne skills to CPT `novamira_skill` i również wymagają osobnego eksportu, jeśli występują.

## 11. Instrukcja pracy dla kolejnego agenta

Skopiuj do instrukcji nowego projektu, po podmianie nazw i domen:

```text
Domyślny cel: https://nowyprojekt.ddev.site, blog ID ustalony z WP.
Dodatkowy blog wybieraj tylko na wyraźne polecenie użytkownika.

Przed pracą z WordPressem:
1. Sprawdź obecność narzędzi właściwego serwera MCP.
2. Wywołaj mcp-adapter-discover-abilities.
3. Pobierz mcp-adapter-get-ability-info dla każdej używanej ability.
4. Wykonuj operację zgodnie z aktualnym schematem.
5. Zapisuj dane tylko w zakresie polecenia użytkownika.
6. Po zapisie odczytaj zmienione dane i sprawdź właściwy blog.

Kod motywu zmieniaj w repozytorium. Nie zastępuj tego zdalnymi zapisami plików.
Nie zmieniaj, nie wyłączaj i nie usuwaj pluginu Novamira ani jego poświadczeń
podczas rutynowej pracy. Nie pokazuj sekretów w wynikach narzędzi i dokumentach.
Nie przełączaj istniejącego Pi Application Password na OAuth w ramach
zwykłej diagnostyki. Brak narzędzi nie dowodzi błędu autoryzacji.
```

Wzorzec w repozytorium: `.pi/skills/mcp-novamira/SKILL.md` i sekcja MCP w `AGENTS.md`. Trzeba dostosować również rozróżnienie klienta Codex i Pi; stara instrukcja ma charakter projektu WFOŚiGW.

Przykład wywołania po discovery i odczytaniu schematu:

```json
{
  "ability_name": "novamira/execute-php",
  "parameters": {
    "code": "return ['blog_id' => get_current_blog_id(), 'home' => home_url(), 'environment' => wp_get_environment_type()];"
  }
}
```

To argumenty narzędzia `mcp-adapter-execute-ability`, nie kompletna koperta JSON-RPC. PHP przekazuje się bez `<?php`, wynik przez `return`. Nie używaj `exit()` ani `die()`. Kod pluginu nakłada limit wykonania `30 s`, niezależny od wyższego globalnego limitu PHP DDEV.

## 12. Procedura odbioru konfiguracji w nowym projekcie

1. `ddev describe`: kontenery działają, właściwe hosty i HTTPS.
2. `wp site list` dla multisite: właściwe ID, domeny i ścieżki.
3. Plugin Novamira jest aktywny na zamierzonym blogu i ma bundled vendor.
4. `novamira_is_enabled()` zwraca `true`; zapis domeny odpowiada `home_url()`.
5. Konto operatora ma wymagane uprawnienia; w multisite jest superadministratorem.
6. Dla Codex: dokumenty `.well-known` zwracają JSON i prawidłowe URL, a logowanie OAuth kończy się zgodą dla właściwego hosta.
7. Dla Pi: most uruchamia się przez `npx`, ufa certyfikatowi, ma właściwy URL, login, Application Password i `OAUTH_ENABLED=false`.
8. Klient widzi trzy narzędzia wejściowe; discovery i ability-info działają.
9. Odczytowy przykład z poprzedniego rozdziału zwraca właściwy blog i URL.
10. Jeśli skonfigurowano dwa hosty, powtórz odbiór dla każdego osobno.

Nie twórz testowych stron ani nie zmieniaj ustawień witryny tylko w celu sprawdzenia połączenia. Do odbioru transportu wystarczy odczyt.

### Polecenia do odczytu stanu

Zamień host przed uruchomieniem:

```bash
ddev exec wp --url=https://nowyprojekt.ddev.site plugin list --status=active
ddev exec wp --url=https://nowyprojekt.ddev.site option get novamira_ai_abilities_enabled
ddev exec wp --url=https://nowyprojekt.ddev.site option get novamira_ai_abilities_domain
ddev exec wp --url=https://nowyprojekt.ddev.site eval 'echo novamira_is_enabled() ? "enabled" : "disabled";'
ddev exec wp --url=https://nowyprojekt.ddev.site eval 'echo wp_json_encode(array_keys(wp_get_abilities()));'
```

Nie wykonuj szerokiego eksportu `novamira_*` z bazy do logów: namespace zawiera również prywatne klucze OAuth.

## 13. Diagnostyka według warstwy

| Objaw | Co sprawdzić |
| --- | --- |
| Docker socket niedostępny / brak kontenerów | działanie Dockera, DDEV i uprawnienia procesu; to wcześniejsza warstwa niż MCP |
| Narzędzi nie ma w sesji | konfiguracja właściwego klienta, nazwa serwera, przeładowanie i zaufanie projektu |
| Pi widzi BIP, a nie widzi głównej witryny | w badanym stanie tylko BIP miał znalezioną definicję Pi; dodaj właściwą definicję |
| Błąd TLS | zaufanie do CA DDEV w środowisku klienta, opcja Node i DNS |
| `.well-known` zwraca `403` | wyjątek w regule blokującej ukryte ścieżki nginx |
| Endpoint MCP zwraca `404` | aktywacja pluginu na tym blogu, AI Abilities, blokada domeny, vendor i rewrite |
| `novamira_mcp_adapter_unavailable` / błąd 500 | kompletność paczki i komunikaty dependency error pluginu |
| `401` dla Pi | hasło aplikacyjne, login, URL `/novamira`, `OAUTH_ENABLED=false`, przepuszczenie Authorization do PHP |
| `401` i challenge dla `/novamira-oauth` bez tokenu | może być prawidłowym początkiem OAuth; sprawdź metadata i wykonaj login klienta |
| `403` podczas wykonania ability | uprawnienia konta, superadmin multisite, stan pluginu i reguły abilities |
| GET endpointu zwraca `405` | ta wersja transportu nie implementuje GET SSE; użyj klienta MCP |
| Login BIP wraca na formularz | domena cookies i właściwy host; sprawdź `COOKIE_DOMAIN=''` |
| Po imporcie bazy brak abilities | stara domena w `novamira_ai_abilities_domain`; ponownie włącz na lokalnym hoście |
| Narzędzie działa na niewłaściwej treści | sprawdź `get_current_blog_id()` i host endpointu |
| WP-CLI abilities nie występują | dostępność `wp`, `exec`, `proc_open` w PHP serwera |
| Zmiana w panelu nie pojawia się w kliencie | przeładuj sesję i discovery; sprawdź cache, reguły i preferencje funkcji |
| Zamiast JSON pojawiają się warnings PHP | logowanie błędów bez display; usuń przyczynę w odpowiednim pluginie |

Podczas audytu WP-CLI na blogu 1 wypisywał ostrzeżenie deprecated z WP Mail SMTP (`QuickConnectUsage.php:393`, nullable parameter). Polecenia zakończyły się sukcesem. Nie modyfikowano tego pluginu. Nie potwierdzono, czy ostrzeżenie występuje także w odpowiedziach HTTP MCP.

## 14. Co przenosić, a czego nie kopiować

**Do nowego projektu przenieś:** właściwą wersję pluginu z vendor, dostosowany nginx, parametry środowiska, strukturę konfiguracji klienta i instrukcje agentów. Ustaw AI Abilities dla nowej domeny oraz utwórz uprawnione konto i nowe poświadczenia.

**Nie kopiuj jako konfiguracji przenośnej:** hasła aplikacyjnego WFOŚiGW, kluczy prywatnych i tokenów OAuth, hashy zgód Pi, identyfikatora urządzenia, starych sesji, loginu operatora ani sztywno zapisanych domen WFOŚiGW. Jeśli wymagany jest import danych WordPressa, zastosuj projektowy przepływ test → backup Updraft → local, z kontrolą URL multisite po odtworzeniu.

Uzupełnij `.gitignore` nowego projektu co najmniej o prywatne konfiguracje i runtime, zgodnie z wybranym modelem dystrybucji pluginu:

```gitignore
wp-config-local.php
/.pi/mcp-adapter.json
/.pi/mcp-secrets.local.json
/wp-content/novamira-sandbox/
```

W WFOŚiGW ignorowane są ponadto `/wp-content/plugins/novamira/` oraz `/.ddev/nginx_full/nginx-site.conf`. Jeżeli zachowasz te wyjątki, zapewnij osobne dostarczenie obu elementów przy każdej instalacji. Nie zakładaj, że pipeline GitLab wdroży plik, którego nie ma w repozytorium.

## 15. Zakres audytu i granice potwierdzenia

Przejrzano pliki projektu, kod Novamira i bundled adaptera, lokalny nginx, konfiguracje globalne Codex/Pi oraz kod i dokumentację lokalnego `pi-mcp-adapter`.

Przez działające DDEV i WP-CLI potwierdzono wersje runtime, topologię multisite, aktywację pluginów, opcje Novamira obu blogów, dostępność Application Passwords, uprawnienia konta Pi, nazwy jego haseł aplikacyjnych, rejestrację abilities i tras na blogu 1.

Nie uruchamiano nowego loginu OAuth, nie odczytywano tokenów ani prywatnych kluczy, nie inicjowano procesu mostu Pi i nie wykonano pełnego handshake HTTP z uwierzytelnionym klientem MCP. Obecna sesja nie udostępniała narzędzi `wfosigw_local`/`bip_wfosigw_local`; użyto odczytu przez WP-CLI. Zarejestrowane trasy i poprawne opcje nie są równoznaczne z potwierdzonym połączeniem całego klienta.

Nie zmieniano konfiguracji witryn, poświadczeń ani kodu aplikacji. Ten dokument oraz dołączony nginx pozwalają odtworzyć konfigurację połączenia; identyczna treść projektowego designu, własnych skills i danych WordPressa wymaga osobnego eksportu.

### Źródła lokalne

- `AGENTS.md`, `.gitignore`, `.ddev/config.yaml`, `.ddev/php/php.ini`.
- `.ddev/nginx_full/nginx-site.conf`, `wp-config.php`, `wp-config-local.php.example` i wybrane niesekretne ustawienia `wp-config-local.php`.
- `.pi/skills/mcp-novamira/SKILL.md`, `.pi/settings.json`.
- `~/.codex/config.toml`, `~/.pi/agent/settings.json`, `~/.pi/agent/mcp-adapter.json`.
- `~/.pi/agent/mcp-project-approvals.json`, `mcp-onboarding.json` — wyłącznie weryfikacja historycznej obecności konfiguracji.
- `wp-content/plugins/novamira/novamira.php`, `README.md`, `includes/compatibility.php`, `includes/helpers.php`, `includes/connect-page.php`.
- `includes/abilities/bootstrap.php`, `includes/abilities/execute-php.php`, `includes/features/manifest.php`, `includes/features/state-store.php`, `includes/instructions-admin.php` w katalogu pluginu.
- `includes/oauth/bootstrap.php`, `schema.php`, `keys.php`, `server-factory.php`, `endpoints/discovery.php`, `middleware.php` w katalogu pluginu.
- `vendor/novamira/mcp-adapter/includes/Transport/HttpTransport.php` i `includes/Abilities/` w katalogu pluginu.
- Lokalny `pi-mcp-adapter` 5.3.0: `package.json`, `config.ts`, `utils.ts`, `README.md`, `docs/configuration.md`, `docs/servers.md`.

### Odciski istotnych plików źródłowych

SHA-256 pozwala rozpoznać dokładnie badane pliki, ale nie zastępuje archiwum całego pluginu:

```text
8b0368a70616bcb5b55d3c2e289c6fc3862ac685a8591baacc35468e194540ea  wp-content/plugins/novamira/novamira.php
abce933fdc8ab772e1d8cccf5611f86547fa586811a45a9ab35df148ac70e795  wp-content/plugins/novamira/includes/helpers.php
1d81b99891ef31d5a56f62893f6851d384e0895112cfdaca09bb687cad8f24e5  .ddev/nginx_full/nginx-site.conf
```

## Załącznik: pełna lokalna konfiguracja nginx

Poniżej należy zachować zawartość pliku źródłowego; `root /var/www/html` odpowiada docroot `.` w badanym DDEV.

```nginx
# ddev wordpress config
# https://developer.wordpress.org/advanced-administration/server/web-server/nginx/

# This project-owned configuration is preserved by DDEV across restarts.
# See https://docs.ddev.com/en/stable/users/extend/customization-extendibility/#custom-nginx-configuration

# Much of this config is adapted from
# https://codex.wordpress.org/Nginx

server {
    listen 80 default_server;
    listen 443 ssl default_server;

    root /var/www/html;

    ssl_certificate /etc/ssl/certs/master.crt;
    ssl_certificate_key /etc/ssl/certs/master.key;

    include /etc/nginx/monitoring.conf;

    index index.php index.htm index.html /ddev-webserver-403-error;

    # Disable sendfile as per https://docs.vagrantup.com/v2/synced-folders/virtualbox.html
    sendfile off;
    error_log /dev/stdout info;
    access_log /var/log/nginx/access.log;

    # From wordpress demo global_restrictions.conf
    # Global restrictions configuration file.
    # Designed to be included in any server {} block.
    location = /favicon.ico {
        access_log off;
        log_not_found off;
    }

    # Deny all attempts to access hidden files such as .htaccess, .htpasswd, .DS_Store (Mac).
    # Keep logging the requests to parse later (or to pass to firewall utilities such as fail2ban)
    location ~* /\.(?!well-known\/) {
        deny all;
    }

    # Deny access to any files with a .php extension in the uploads directory
    # Works in sub-directory installs and also in multisite network
    # Keep logging the requests to parse later (or to pass to firewall utilities such as fail2ban)
    location ~* /(?:uploads|files)/.*\.php$ {
        deny all;
    }

    location / {
        absolute_redirect off;
        # This is cool because no php is touched for static content.
        # include the "?$args" part so non-default permalinks doesn't break when using query string
        try_files $uri $uri/ /index.php?$args;
    }

    # pass the PHP scripts to FastCGI server listening on socket
    location ~ \.php$ {
        try_files $uri =404;
        fastcgi_split_path_info ^(.+\.php)(/.+)$;
        fastcgi_pass unix:/run/php/php-fpm.sock;
        fastcgi_buffers 16 16k;
        fastcgi_buffer_size 32k;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        fastcgi_param SCRIPT_NAME $fastcgi_script_name;
        fastcgi_index index.php;
        include fastcgi_params;
        fastcgi_intercept_errors off;
        # fastcgi_read_timeout should match max_execution_time in php.ini
        fastcgi_read_timeout 10m;
        fastcgi_param SERVER_NAME $host;
        fastcgi_param HTTPS $fcgi_https;
        # Pass the X-Accel-* headers to facilitate testing.
        fastcgi_pass_header "X-Accel-Buffering";
        fastcgi_pass_header "X-Accel-Charset";
        fastcgi_pass_header "X-Accel-Expires";
        fastcgi_pass_header "X-Accel-Limit-Rate";
        fastcgi_pass_header "X-Accel-Redirect";
    }

    # Expire rules for static content

    # Media: images, icons, video, audio, HTC
    location ~* \.(jpg|jpeg|gif|png|ico|cur|gz|svg|svgz|mp4|ogg|ogv|webm|webp|htc)$ {
        expires max;
        log_not_found off;
        try_files $uri /index.php$is_args$args;
    }
    location ~* \.(js|css)$ {
        expires -1;
        log_not_found off;
    }
    include /etc/nginx/common.d/*.conf;
    include /mnt/ddev_config/nginx/*.conf;
}
```
