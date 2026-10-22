# Test integracyjny Playwright

Z katalogu głównego repozytorium:

```sh
make test-playwright
```

Test działa jako UID/GID 1000:1000 w trwałej VM Debian 13 amd64 Incus/QEMU/KVM.
Host wymaga przygotowanego Incus, rsync i narzędzi opisanych w
[instrukcji testowania](../../../../docs/testing.md), ale nie Cargo, Node ani
Dockera. VM wymaga Docker Engine, Compose, GNU timeout, Rust i **Node.js 24 LTS**:
Node poza kontenerem jest niezbędny jako klient MCP w etapie zainstalowanego
adaptera. Provisioning dostarcza te zależności wewnątrz VM.

Runner w gościu zawsze wykonuje istniejące polecenia:

```sh
docker build --tag ai-profiles-playwright:dependency-smoke mcp/images/playwright
sh mcp/images/playwright/tests/run.sh ai-profiles-playwright:dependency-smoke
```

Nie trzeba ręcznie uruchamiać usług ani przeglądarki. Test używa produkcyjnego
Compose z testowym nadpisaniem obrazu/entrypointu. Sprawdza MCP stdio, schematy
i roots, Chromium, axe 4.14.0, naruszenia button-name/image-alt, screenshoty
względne/bezwzględne/automatyczne, trwałość init script po nawigacji,
HTTPS i cookie uwierzytelnienia. Następnie rzeczywiste `make install CLIENT=opencode PROFILE=react`
i zainstalowany adapter wykonują pełny drugi etap. Nie ma mockowania adaptera.

Czysty projekt bez `reports/` i `local.json` uruchamia dystrybuowany launcher
z cwd `/`, w ścieżce ze spacjami i przecinkiem. Test sprawdza automatyczne
przygotowanie raportów, HTTP Basic Auth (401/200, błędne hasło/port, brak
Authorization na drugim origin i po redirect), hostowy localhost oraz mounty
RO hosts/resolv. Syntetyczny resolver obsługuje `app.corp.test` przez testowy
overlay, bez zmian sieci i plików hosta VM — nie dowodzi działania firmowego VPN.
Automatyczny CA z syntetycznego CAROOT poza projektem umożliwia HTTPS także z
Basic Auth/storage state; obcy CA jest odrzucany. Inspect sprawdza pojedynczy
mount publicznego CA RO, a kontrola prywatności wyklucza credentials i sentinel
klucza z protokołu, logów, stanu i raportów. Są scenariusze braku i wadliwego CA.
Konfigurację wszystkich trzech klientów sprawdzają testy lifecycle Rust w
`make test-all`; integracja Chromium nie uruchamia rzeczywistych aplikacji klientów.

Kontener wnioskuje UID/GID 1000:1000 z właściciela świeżego katalogu raportów.
Mount projektu jest read-only, zagnieżdżony mount raportów zapisywalny; tylko
tymczasowy katalog raportów otrzymuje testowe uprawnienia. Socket Docker nigdy
nie jest montowany. `network_mode: host` oznacza przestrzeń sieciową VM.

Świeże CA/certyfikaty i syntetyczny auth state powstają przez OpenSSL obrazu;
TLS pozostaje weryfikowany. Każdy przebieg ma osobny TMPDIR. JSON MCP/axe oraz
PNG trafiają do `.test-vm/artifacts/<run>/export/` także po błędzie; klucze,
auth state i cały tymczasowy projekt nie są eksportowane. Scratch pozostaje
w gościu do świadomego usunięcia. Cargo target/registry i warstwy Docker
przetrwają kolejne wywołania oraz `make test-vm-stop`; testy nie są pomijane.

Dotychczasowe deadline pozostają: MCP 30 s, harness 120 s, zewnętrzny Docker
180 s. Runner usuwa tylko własne unikalnie nazwane kontenery także po błędzie
i przerwaniu. Obraz nie jest publikowany. Test amd64 nie weryfikuje arm64.
