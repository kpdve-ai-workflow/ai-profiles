Herdr zapewnia sterowanie agentami, ale sam nie realizuje zasad ich współpracy. Nasza implementacja zbudowała konkretny workflow na jego mechanizmach.

Przykładowo: Herdr potrafi uruchomić Codexa, wysłać mu pytanie i poczekać, aż agent przestanie pracować. Zakończenie pracy nie mówi jednak, czy agent przygotował kompletny raport, odpowiedział na właściwe zadanie albo zatrzymał się bez wyniku.

| Potrzeba | Co zapewnia Herdr | Co dodaliśmy |
|---|---|---|
| Uruchomienie agentów | Panele i osobne procesy | Dobór konfiguracji Braina i Researchera |
| Delegacja | Przekazanie pytania do agenta | Narzędzie research, które uruchamia lub wykorzystuje właściwego Researchera |
| Odbiór wyniku | Odczyt terminala i stanu agenta | Osobny kanał raportu, bez historii skanowania |
| Rozpoznanie zakończenia | Stan typu idle, done, blocked | Sprawdzenie, czy otrzymaliśmy raport dla danego zadania |
| Timeout | Zakończenie oczekiwania błędem | Zachowanie zadania i możliwość odzyskania późniejszego wyniku bez ponownej wysyłki |
| Kolejne zlecenia | Możliwość ponownego użycia procesu | Blokadę równoległych zleceń i odtwarzanie Researchera po limicie zadań |
| Sprzątanie | Zamykanie paneli | Śledzenie paneli utworzonych przez konkretną sesję |

Najważniejszy zysk to zamiana niskopoziomowych poleceń Herdr w jedną operację dostępną dla Braina:

> „Zbadaj to pytanie i zwróć raport”.

Brain nie musi za każdym razem samodzielnie ustalać, jak znaleźć proces, uruchomić go, sprawdzić zajętość, wysłać pytanie i odebrać wynik. Te powtarzalne czynności wykonuje kod według ustalonych reguł.

Moglibyśmy opisać całą procedurę w instrukcjach i pozwolić Brainowi wykonywać bezpośrednio komendy Herdr. Wtedy jednak poprawność tej procedury zależałaby za każdym razem od zachowania modelu. Implementacja wymusza część zasad technicznie: np. odrzuca równoległe zlecenie, nie nadpisuje raportu i nie wysyła automatycznie ponownie zadania po niepewnym dostarczeniu.

Pozostają też granice: model nadal decyduje, kiedy potrzebuje researchu, a kod nie potwierdza prawdziwości raportu. Nie gwarantuje też oszczędności tokenów. Zapewnia przewidywalny mechanizm wykonania delegacji i przekazania wyniku; Herdr zapewnia procesy, na których ten mechanizm działa.