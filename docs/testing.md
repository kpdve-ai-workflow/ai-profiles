# Testy w trwałej VM Incus

Testy Rust i pełny test integracyjny Playwright działają wyłącznie w prawdziwej
VM QEMU/KVM Debian 13 Trixie amd64. Host nie uruchamia Cargo, Node ani Dockera.
Wymagania te dotyczą testowania, nie lokalnego `make install CLIENT=opencode PROFILE=… PROJECT=…`.
Instalator nadal wymaga Rust/Cargo 1.85+, Git i Make.

## Projektowanie i jakość testów

Metodyka `test-gap-discovery` obowiązuje przy projektowaniu, dodawaniu,
modyfikowaniu i ocenie testów, także bez Brain. Przed taką pracą przeczytaj ten
dokument i wczytaj skill, jeśli jest dostępny. Jeśli go brakuje, zgłoś
niedostępność; poniższe zasady nadal obowiązują i nie mogą być zastąpione
dowolnym podejściem.

1. Ustal obserwowalne wymaganie i niezależne źródło oczekiwanego wyniku:
   wymagania, kontrakt, standard, sprawdzony przykład lub niezmiennik.
2. Wyprowadź scenariusze przed kopiowaniem oczekiwań z implementacji. Czytanie
   kodu może pomóc znaleźć punkty testowania, ale aktualny wynik programu nie
   jest sam w sobie dowodem poprawności.
3. Dobieraj przypadki według ryzyka: granice, błędne dane, sekwencje, ponowienia,
   częściowe awarie, izolacja i integracje — tylko tam, gdzie mają znaczenie.
4. Wybierz właściwy poziom testu, dane, izolację i granice mockowania. Nie
   zastępuj mockiem zachowania, którego poprawność test ma potwierdzić.
5. Wskaż wiarygodny błędny wariant zachowania, który test powinien wykryć,
   oraz asercję, która go ujawni.
6. Uruchom testy zgodnie z polityką repozytorium i oceń rzeczywisty wynik.
7. Przy niepowodzeniu rozróżnij błąd produktu, błędne oczekiwanie, wadliwe dane
   testowe, niedeterministyczność i problem infrastruktury, zamiast automatycznie
   dopasowywać test do implementacji.

Dla nowego lub naprawianego zachowania stosuj **test-first**: niezależne
oczekiwanie → test → uzasadnione **RED** przed zmianą implementacji → minimalna
implementacja → **GREEN** → wymagane kontrole regresji. RED ma wynikać
z brakującego lub błędnego zachowania, nie z błędu środowiska. Test istniejącego,
poprawnego zachowania może być od razu GREEN; sprawdź jego zdolność wykrywania
realistycznej regresji, ale nie psuj kodu, aby wymusić RED. Nierozstrzygnięta
sprzeczność oczekiwań z kontraktem wymaga zatrzymania i zgłoszenia.

Nie wymagamy odkrycia błędu przez każdy test, minimalnej liczby przypadków ani
eksperymentów mutacyjnych. Mutacje są opcjonalne; wymagają bezpiecznej izolacji
(jednorazowej kopii lub odrębnego worktree), odpowiedniej wyraźnej zgody i kontroli
sprzątania. Nie wolno zmieniać wymagań, usuwać zasadnych asercji ani dodawać
skipów tylko po to, by uzyskać GREEN.

Mockowane wywołanie procesu nie dowodzi działania prawdziwego programu.
Kontrola orkiestracji nie zastępuje rzeczywistej integracji. W Rust samo dodanie
pliku do `core/tests/` nie gwarantuje wykonania: `core/Cargo.toml`
ustawia `autotests = false`, a pliki są jawnie dołączane jako moduły testowe
w `core/src/lib.rs`. Sprawdź rejestrację modułu i potwierdź w wynikach
uruchomienie nowych testów, nie tylko sukces całego polecenia.

### Wymagana weryfikacja przed akceptacją

Zmiana zachowania wymaga adekwatnej ochrony testowej oraz poniższej bramki.
Polecenia Make uruchamiaj z katalogu głównego repozytorium; orkiestrują testy
Rust i Playwright w VM, bez lokalnego fallbacku. Lekką kontrolę orkiestracji
`bash tests/vm/check.sh` można wykonać na hoście; nie testuje ona aplikacji w VM.

| Zakres zmiany | Weryfikacja przed akceptacją |
| --- | --- |
| Zachowanie `core` | `make validate-rust-migration` |
| Integracja/obraz Playwright | `make test-playwright` |
| Wspólne zmiany obejmujące oba obszary | `make test-all` |
| Orkiestracja VM | `bash tests/vm/check.sh` oraz `make test-all` |
| Workflow agentów Herdr | `make validate-rust-migration` w VM, `bash tests/agents/check-make.sh` i rzeczywista próba produkcyjna `bash tests/agents/live.sh` na hoście z Herdr/Codex |
| Wyłącznie dokumentacja/instrukcje | Kontrola treści, odnośników i spójności; bez udawania weryfikacji aplikacji |

`make test` służy również iteracyjnej weryfikacji Rust. Szersza bramka może już
obejmować wymagane testy (`make test-all` obejmuje bramkę Rust i Playwright,
a `make validate-rust-migration` obejmuje `make test`); nie uruchamiaj ich
ponownie wyłącznie dla formalności. Dla nowych obszarów ustal kontrakt
weryfikacji na podstawie rzeczywistych narzędzi projektu; nie wymyślaj komend.

Zapisuj rzeczywiście wykonane polecenia, ich rezultaty i ograniczenia
w istniejącym planie lub podsumowaniu, wraz z uzasadnionymi scenariuszami.
Nie twórz obowiązkowych osobnych raportów dla każdego testu. Oddzielaj status
implementacji od weryfikacji: „kod zmieniony” nie znaczy „zachowanie zweryfikowane”.
Brak środowiska, uprawnień lub możliwości wymaganej weryfikacji oznacza blokadę,
nie sukces ani nieadekwatność testów. Zatrzymaj pracę, zachowaj zmiany i zgłoś
dokładny bloker oraz faktyczne wyniki; nie akceptuj zadania (checkbox pozostaje
niezaznaczony, jeśli jest używany). Nie osłabiaj testów, nie pomijaj wymaganych
kontroli ani nie obchodź izolacji. Zakaz lokalnych testów Rust i Playwright nie
dotyczy lokalnych operacji produkcyjnych opisanych dalej.

## Przygotowanie hosta

Pierwsza implementacja obsługuje tylko Linux x86_64/amd64. Użytkownik musi
świadomie zainstalować i uruchomić lokalny Incus z obsługą VM/QEMU, włączyć
wirtualizację sprzętową (także nested virtualization na hoście będącym VM),
zapewnić dostęp do `/dev/kvm`, zainicjalizować daemon oraz przygotować storage
pool i zarządzany bridge z IPv4 NAT/DHCP, DNS i działającym wyjściem do Internetu.
Potrzebne są Make, Bash, rsync, tar, jq, util-linux (`flock`), GNU coreutils
i uprawnienia do lokalnego socketu Incus. Testy nie instalują pakietów hosta,
nie tworzą sieci/puli i nie zmieniają grup ani globalnych profili Incus.

**Dostęp administracyjny do socketu Incus jest wysoce uprzywilejowany** (w praktyce
pozwala przejąć hosta). Nie nadawaj go niezaufanym użytkownikom. Orkiestracja używa
`--force-local --project default`, niezależnie od aktualnego remote/projektu CLI.
Wyjątkiem składni CLI jest `incus query`: wymusza lokalny daemon przez
`--force-local`, a projekt zasobu wskazuje parametrem URL `?project=default`.

Jeśli dystrybucja używa innej ścieżki socketu niż domyślna CLI (np. Bluefin),
ustaw ją w terminalu przed wywołaniem Make:

```sh
export INCUS_SOCKET=/run/incus/unix.socket
```

Właściwą ścieżkę pokazuje `systemctl cat incus.socket` w `ListenStream`.

### Dokładna konfiguracja lokalnego Incus (Bluefin/Fedora, systemd)

Poniższa konfiguracja odpowiada przygotowaniu hosta do tego repozytorium.
Zakłada już zainstalowane Incus i QEMU oraz dostępne `/dev/kvm`. To świadoma,
jednorazowa konfiguracja administracyjna hosta, nie część skryptów testowych.

**1. Uruchom socket i sprawdź dostęp do daemona:**

```sh
sudo systemctl enable --now incus.socket
export INCUS_SOCKET=/run/incus/unix.socket
incus --force-local info
```

Na Bluefin socket należy do grupy `incus-admin`. Jeśli występuje odmowa dostępu,
sprawdź `id` i `systemctl cat incus.socket`; nadanie dostępu powinien świadomie
wykonać administrator. Po zmianie grup trzeba ponownie się zalogować. Nie
naprawiaj tego przez `chmod 666` ani uruchamianie całych testów przez sudo.
`INCUS_SOCKET` obowiązuje w bieżącym terminalu; po otwarciu nowego ustaw ją
ponownie albo świadomie dodaj export do własnej konfiguracji powłoki.

**2. Sprawdź istniejącą konfigurację i zajęte podsieci:**

```sh
incus --force-local admin init --dump
incus --force-local --project default list
incus --force-local storage list
incus --force-local --project default network list
ip -4 route show
```

Jeśli masz już pulę i odpowiednią sieć, **nie uruchamiaj ponownie inicjalizacji**.
Wykorzystaj je przez `TEST_VM_POOL` i `TEST_VM_NETWORK`. Preseed poniżej jest
przeznaczony dla świeżego, nieskonfigurowanego Incus: tworzy zasoby i ustawia
domyślny profil. Nie stosuj go w ciemno do instalacji używanej przez inne projekty.

**3. Na świeżej instalacji zastosuj gotowy preseed zamiast wybierać opcje
interaktywnie:**

Sprawdź wcześniej, czy `10.203.77.0/24` nie koliduje z LAN, VPN, Dockerem ani
innymi sieciami. Jeżeli koliduje, zmień `ipv4.address` na wolną prywatną podsieć.

```sh
incus --force-local admin init --preseed <<'YAML'
config: {}
networks:
  - name: incusbr0
    type: bridge
    config:
      ipv4.address: 10.203.77.1/24
      ipv4.nat: "true"
      ipv6.address: none
storage_pools:
  - name: default
    driver: dir
    config: {}
profiles:
  - name: default
    description: Default Incus profile
    config: {}
    devices:
      eth0:
        name: eth0
        network: incusbr0
        type: nic
      root:
        path: /
        pool: default
        type: disk
YAML
```

Wybrane ustawienia:

| Element | Wartość i znaczenie |
| --- | --- |
| Klaster | Nie; pojedynczy lokalny daemon. |
| API przez sieć | Wyłączone; dostęp przez lokalny socket, bez otwierania portu 8443. |
| Pula `default` | Backend `dir`, bez wskazywania urządzenia i bez formatowania dysków. Dane pod katalogiem Incus na filesystemie hosta (typowo `/var/lib/incus/storage-pools/default`). |
| Sieć `incusbr0` | Zarządzany bridge, brama `10.203.77.1/24`, IPv4 NAT; domyślne DHCP i DNS Incus włączone. |
| IPv6 | Wyłączone dla tej sieci. |
| Interfejs fizyczny | Nie jest przyłączany do bridge; VM wychodzi do Internetu przez NAT hosta. |
| Profil `default` | Dysk z puli `default` i karta `eth0` w `incusbr0`. Orkiestrator testów używa `--no-profiles` i wskazuje te zasoby jawnie, więc nie zależy od tego profilu. |

Pula katalogowa korzysta z wolnego miejsca hosta; nie rezerwuje automatycznie
64 GiB. Rozmiar dysku konkretnej VM ustawia dopiero `make test-vm-up`.

**3a. Na hoście z firewalld zezwól na DHCP/DNS i ruch wychodzący z bridge'a:**

Na Bluefin/Fedora samo utworzenie bridge'a i NAT przez Incus może nie wystarczyć.
Nieprzypisany bridge trafia pod reguły domyślnej strefy; blokada DHCP powoduje
brak IPv4 w VM, a APT zgłasza potem `Temporary failure resolving deb.debian.org`.
Nie naprawiaj tego przez wyłączanie zapory ani wpisywanie publicznego DNS
w gościu pozbawionym adresu i trasy domyślnej.

Poniższa jednorazowa konfiguracja tworzy dedykowaną strefę `ai-profiles`
wyłącznie dla `incusbr0` oraz politykę `ai-profiles-out`. Najpierw sprawdź
`firewall-cmd --get-active-zones` i `firewall-cmd --permanent --get-policies`.
Nie stosuj jej bez inspekcji, jeśli te nazwy już istnieją lub bridge ma własne
reguły. Użyj faktycznej nazwy bridge'a, jeżeli jest inna niż `incusbr0`.

```sh
sudo firewall-cmd --permanent --new-zone=ai-profiles
sudo firewall-cmd --permanent --zone=ai-profiles --add-service=dhcp
sudo firewall-cmd --permanent --zone=ai-profiles --add-service=dns
sudo firewall-cmd --permanent --zone=ai-profiles --add-interface=incusbr0
sudo firewall-cmd --permanent --new-policy=ai-profiles-out
sudo firewall-cmd --permanent --policy=ai-profiles-out --add-ingress-zone=ai-profiles
sudo firewall-cmd --permanent --policy=ai-profiles-out --add-egress-zone=ANY
sudo firewall-cmd --permanent --policy=ai-profiles-out --set-target=ACCEPT
sudo firewall-cmd --check-config
sudo firewall-cmd --reload
```

Wykonuj kolejne polecenie tylko po sukcesie poprzedniego. `--reload` stosuje
konfigurację trwałą z zachowaniem stanu połączeń; przed jego użyciem sprawdź,
czy inne potrzebne reguły firewalld nie istnieją wyłącznie w konfiguracji runtime.
Nie używaj `--complete-reload` ani zbiorczego `--runtime-to-permanent`.

Strefa pozwala gościowi korzystać z DHCP i DNS hosta, ale nie otwiera mu
wszystkich usług hosta jak strefa `trusted`. Polityka pozwala na forwarding
z tej strefy do innych stref (`ANY` nie oznacza `HOST`). NAT pozostaje
zarządzany przez Incus. Nie zmieniamy stref interfejsu fizycznego, Dockera ani
libvirt. Jest to świadoma konfiguracja hosta, nigdy automatyczny krok testów.

Weryfikacja:

```sh
firewall-cmd --zone=ai-profiles --list-all
firewall-cmd --policy=ai-profiles-out --list-all
incus --force-local --project default exec NAZWA_VM -- ip -4 address
incus --force-local --project default exec NAZWA_VM -- ip -4 route
incus --force-local --project default exec NAZWA_VM -- timeout 10 getent ahostsv4 deb.debian.org
```

Jeśli VM uruchomiono przed naprawą zapory, odnowienie DHCP przez
`networkctl renew enp5s0` wewnątrz tego obrazu przywraca adres bez restartu VM
(sprawdź nazwę interfejsu przez `ip link`). Po potwierdzeniu łączności wykonaj
`make test-vm-reprovision`, aby naprawić przerwany provisioning bez usuwania
dysku. Sama naprawa sieci nie ponawia zakończonych błędem skryptów cloud-init.

**4. Zweryfikuj wynik i przygotuj VM:**

```sh
incus --force-local storage list
incus --force-local --project default network list
incus --force-local query /1.0 | jq '.environment | {driver, addresses}'
TEST_VM_POOL=default TEST_VM_NETWORK=incusbr0 make test-vm-up
```

Oczekiwane: pula `default` ma stan `CREATED`, zarządzana sieć `incusbr0` ma stan
`CREATED`, driver zawiera `qemu`, a `addresses` jest pustą listą przy niewystawionym
API. Stan `CREATED` sieci nie dowodzi jeszcze dostępu VM do Internetu; potwierdza
go dopiero udany provisioning. Po przygotowaniu VM wykonaj `make test-all`.
Przy jednej puli i jednej pasującej sieci zmienne `TEST_VM_POOL` i
`TEST_VM_NETWORK` można pominąć.

Jeżeli daemon nadal nie startuje:

```sh
systemctl status incus.socket incus.service --no-pager
sudo journalctl -u incus.service -u incus.socket -n 80 --no-pager
```

### Źródło obrazu i zasoby VM

Remote `images` musi wskazywać `https://images.linuxcontainers.org` przez
simplestreams. Przed utworzeniem sprawdzany jest aktualny katalog:

```sh
incus remote list --format json
incus image list images: debian/13/cloud --format json
```

Wybierany jest wyłącznie obraz typu `virtual-machine`, `x86_64`, Debian,
`release=trixie`, `variant=cloud`, alias `debian/13/cloud` lub wariant `/amd64`.
Nie ma fallbacku do kontenera, Debian 12 ani innego systemu. Wariant cloud
dostarcza cloud-init i incus-agent; ich faktyczne działanie jest sprawdzane po
starcie. Fingerprint zostaje zapisany w `.test-vm/image-source`, konfiguracji
VM `user.ai-profiles.image` i metadanych każdego przebiegu. Identyfikator
jest wybierany ponownie dopiero przy tworzeniu nowej VM, nie przy każdym teście.

Domyślnie VM ma **4 vCPU, 8 GiB RAM, dysk 64 GiB**. Przy pierwszym utworzeniu:

```sh
TEST_VM_CPUS=6 TEST_VM_MEMORY_GIB=12 TEST_VM_DISK_GIB=96 make test-vm-up
TEST_VM_POOL=default TEST_VM_NETWORK=incusbr0 make test-vm-up
```

Dozwolone są liczby całkowite: CPU 1–256, RAM 4–1024 GiB, dysk 32–4096 GiB.
Zmienne nie zmieniają już istniejącej VM. Pool i sieć wybierane są automatycznie
tylko przy jednoznacznym wyborze; inaczej trzeba wskazać istniejące zasoby.
Provisioning wymaga filesystemu o rozmiarze co najmniej 90% zamówionego dysku
i 8 GiB wolnego miejsca; pokazuje też rzeczywiste `df`, nie tylko rozmiar dysku.

## Polecenia

```sh
make test-vm-up                # utworzenie/przygotowanie lub wznowienie
make test                     # pełne cargo test --locked --all-targets
make test-playwright          # zawsze docker build, potem istniejący run.sh
make validate-rust-migration  # format, release, Clippy, walidacja, testy Rust
make test-all                 # powyższa bramka, potem Playwright; Rust tylko raz
make test-vm-sync              # sama synchronizacja aktualnego checkoutu
make test-vm-reprovision       # jawne ponowne wykonanie provision.sh w VM
make test-vm-stop              # zatrzymanie bez usuwania dysku/cache
```

Każdy test wznawia własną VM, czeka na agenta i cloud-init, sprawdza marker
udanego provisioningu, synchronizuje źródła i wykonuje testy jako `tester`
UID/GID 1000:1000. Zagnieżdżone `make test` w bramce wykonuje testy bez ponownej
orkiestracji; rozpoznanie gościa wymaga root-owned markera provisioningu,
nie flagi środowiskowej. Wewnętrzne cele Make odmawiają wykonania poza gościem.
Produkcyjne cele `install`, `update`, `uninstall`, `switch`, `status`, `validate`
i `rust-build` nadal działają lokalnie.

Pierwszy start wymaga Internetu: Debian APT, oficjalne pakiety Docker dla Trixie
(Engine, Buildx, Compose), podpisane repozytorium NodeSource dla Node.js 24 LTS,
rustup stable z rustfmt/Clippy. Wersje systemu, Node i minimum Rust są sprawdzane.
Konflikt istniejących UID/GID 1000 zatrzymuje provisioning zamiast osłabiać testy.
Grupa Docker daje uprawnienia równoważne root **wewnątrz gościa**; granicą
izolacji pozostaje VM. Nie udostępniamy hostowego socketu Docker, home, agenta
SSH ani żadnych live mountów. Nie dodajemy SSH do orkiestracji.

Provisioning działa tylko przy tworzeniu albo jawnej komendzie reprovision.
Ponowny provisioning nie czyści cache i nie wykonuje ogólnego upgrade; instaluje
brakujące pakiety/komponenty. Po zmianie skryptu użyj `make test-vm-reprovision`:
sama zmiana user-data nie uruchamia ponownie cloud-init. Błąd pierwszego
provisioningu pozostawia VM do diagnozy; jawna udana naprawa zapisuje osobny
marker pozwalający zachować historyczny błąd cloud-init bez blokowania testów.

## Źródła, cache i izolacja przebiegów

Testy jednostkowe aktywacji używają wstrzykniętych runnerów; domyślny runner
natywnych integracji w testach zgłasza brak CLI. Nigdy nie rejestrują marketplace
w rzeczywistym HOME. Test adaptera Playwright instaluje wybranego klienta do
jednorazowego projektu, pod `.ai-profiles/{client}/`. Wspólne scenariusze
obejmują `opencode`, `claude` i `codex`, każdy z osobnym projektem, stanem i
artefaktami; natywne pliki konfiguracyjne pozostają właściwe dla klienta.

Jedna nieefemeryczna VM na kanoniczną ścieżkę checkoutu/użytkownika/hosta.
Tożsamość w konfiguracji VM jest weryfikowana przed operacjami; kolizja
zatrzymuje wykonanie. Wszystkie operacje mają wspólny nieblokujący `flock`:
równoległe wywołanie kończy się czytelnym błędem.

Źródła (również niezacommitowane i nowe) są kopiowane przez hostowy rsync do
świeżego stagingu, tar bez dereferencji symlinków i `incus file push`; następnie
guest-local rsync z `--delete` uzgadnia `/home/tester/ai-profiles`. Symlinki
pozostają symlinkami, executable bits i `.dockerignore` zostają zachowane.
Nie jest kopiowany hostowy build ani UID/GID; pliki w workspace należą do testera.
Lista `tests/vm/excludes` wyklucza m.in. `.git`, `.opencode`, `.test-vm`, Cargo
config, katalogi zależności, `.env*`, klucze i lokalne ustawienia. Nie umieszczaj
sekretów pod zwykłymi nazwami źródeł; żaden filtr nie rozpozna dowolnego sekretu.
Staging jest usuwany także po błędach. Nie używamy `--copy-links` ani
`--delete-excluded`. Usunięte źródła znikają z workspace.

Na dysku Incus zostają: `/home/tester/.cargo` (registry/git/toolchain),
`/home/tester/ai-profiles/core/target` (debug/release) i obrazy/warstwy Docker.
Build pozostaje w miejscu wymaganym przez wykrywanie payloadów repozytorium.
**Wyniki testów nigdy nie służą do pomijania wykonania**; obraz Playwright jest
budowany przy każdym żądaniu (Docker może wykorzystać warstwy).

## Logi i diagnostyka

Każde wywołanie drukuje unikalną ścieżkę `.test-vm/artifacts/<run>/`.
Zawiera log hosta, log operacji gościa i wyłącznie raporty JSON/HTML oraz PNG
z `reports/`, także po niepowodzeniu. Nie eksportujemy `.private`, kluczy,
auth storage ani całego tymczasowego projektu. TMPDIR i syntetyczne dane są
świeże dla każdego przebiegu w `/home/tester/test-runs/<run>/tmp`.
Scratch pozostaje w gościu do świadomej diagnostyki/usunięcia; może zawierać
wygenerowane sekrety testowe. Nie ma automatycznego prune ani cargo clean.
`.test-vm` to stan/lock/artefakty, **nie dysk VM**; dyskiem zarządza Incus.

Kod błędu testu przechodzi przez exec i logowanie. Błąd eksportu nie maskuje
błędu testu, ale po sukcesie testów powoduje niepowodzenie wywołania.
Ctrl-C/TERM przekazuje TERM tylko grupie procesów własnego przebiegu, aby
istniejący cleanup Playwright usunął własne kontenery. Nie zatrzymuje całej VM.
Przy braku agenta nie ma fallbacku do SSH ani testów lokalnych.

Nazwę VM znajdziesz w `artifacts/<run>/instance` lub `incus list`. Diagnostyka:

```sh
incus exec NAZWA_VM -- cloud-init status --long
incus exec NAZWA_VM -- cat /var/log/cloud-init-output.log
incus exec NAZWA_VM -- df -h /home/tester
incus exec NAZWA_VM -- du -sh /home/tester/.cargo /home/tester/ai-profiles/core/target /home/tester/test-runs
incus exec NAZWA_VM -- docker system df
```

Lekkie kontrole granic orkiestracji: `bash tests/vm/check.sh`. Nie zastępują
prawdziwego `make test-all`. Brak daemona, agenta, uprawnień lub wirtualizacji
jest blokerem, a nie wynikiem testów aplikacji.
