# Cable Management

En native Rust-app för Windows 10/11 (64-bit) som dokumenterar exakt var fysisk utrustning och kablar sitter. Appen fungerar helt lokalt, utan konto eller internet.

## Köra

Starta `CableManagement.exe`. Inga filer behöver ligga bredvid programmet och ingen Rust-installation krävs. Grafikdrivrutinen måste stödja OpenGL; uppdatera drivrutinen om appen inte kan skapa grafikfönstret.

På startsidan kan du skapa en PC eller öppna det inbyggda exemplet: AMX-PC → USB-hubb → webbkamera. Kameran är en etikett på hubbens USB-A 3, inte en egen enhetsbox. Exemplet har två individuella HDMI-portar på datorn.

1. Skapa portförsedda enheter med **Ny enhet** i vänsterspalten.
2. Välj **Redigera**. Namnge och dimensionera höljets boxar eller kilar. Byggstenar kan kombineras och förskjutas i tre dimensioner.
3. Välj en fast sidvy. Dra önskad portikon från högerpaletten och släpp på enhetens synliga yta. Alla 20 porttyper finns i paletten; scrolla för resten. En förhandsvisning visar placeringen. Dragningen aktiverar redigering och nivåvyn automatiskt.
4. Dra porten på ytan, eller ange U/V i millimeter och rotation i grader. **Tillämpa ändringar** bekräftar formulärets värden. Giltiga formulärvärden bevaras även när du sparar, navigerar eller skapar annat innehåll.
5. Dubbelklicka på en port och skriv t.ex. **gamingmus**. Namnet blir en etikett vid porten, utan att en separat enhet skapas. Skapa en portförsedd nod bara när dess anslutningar ska mappas, t.ex. en USB-hubb. Kablar till andra mappade enheter kopplas i inspektören.
6. Växla mellan **Nivå för nivå** och **Hela trädet**. Vänsterlistan går direkt till varje enhet. Trädvyn visar alla grenar och har ingen expandera/kollapsa-funktion. Klick på en korttitel öppnar 3D-vyn; portval visar anslutningen i inspektören.
7. Högerklicka på en placerad port och välj **Delete · Ta bort port**. Både port och anslutning tas bort; Ctrl+Z återställer dem.
8. Spara som `.cable.json`. Återöppning återställer modeller, portar, anslutningar och standardvyer.

Vanliga hierarkikablar är riktade från överordnad enhet till underordnad enhet. Välj datorporten först och hubbens uppströmsport som mål. **Extralänk** tillåter avsiktlig kabeldragning utanför den acykliska hierarkin och visas orange i trädvyn. **Adapter** behövs när kabeln konverterar mellan olika kontaktfamiljer. Det är en dokumentation av manuell kabeldragning, inte elektrisk verifiering av en kabel.

## Kontroller

| Kontroll | Funktion |
|---|---|
| Drag från en portikon i högerpaletten | Placera port på den synliga enhetsytan |
| Dubbelklick på port eller dess etikett | Skriv eller ändra ansluten utrustnings namn |
| Högerklick på port eller dess etikett | Meny med Delete och etikettredigering |
| Vänsterdrag på en port i redigeringsläge | Placera på samma yta |
| Mittenknapp + drag | Panorera modell |
| Scroll eller +/− | Zoom |
| Fram, Bak, Vänster, Höger, Topp, Botten | Mjukt animerade vybyten; modellen roteras enbart genom dessa val |
| Standardvy | Återgå till enhetens sparade standardvy |
| Ctrl+S / Ctrl+Skift+S | Spara / Spara som |
| Ctrl+O / Ctrl+N | Öppna / Nytt projekt |
| Ctrl+Z / Ctrl+Y | Ångra / Gör om |

Porternas mått är nominella kontaktöppningar i mm. Höljesändringar skalar inte porterna. Zoom, markeringar och etiketter gör små portar läsbara utan att ändra deras fysiska proportioner. Portformen representerar kontakten; standardversion och kapacitet skrivs i metadatafältet.

## Lagring och återställning

Formatet beskrivs i [docs/FORMAT.md](docs/FORMAT.md). Sparning validerar dokumentet och bevarar föregående fil som `.bak`. Windows använder `MoveFileExW` för atomisk ersättning; originalet tas aldrig bort före ersättning. Fel lämnar det öppna projektet och originalfilen kvar.

Osparade giltiga ändringar får ett separat återställningsutkast var 15:e sekund i `%LOCALAPPDATA%\CableManagement\recovery.cable.json`. Utkastet kan öppnas från vänsterspalten vid nästa start. En lyckad explicit sparning eller ett uttryckligt beslut att kasta ändringar rensar utkastet. Inget utkast öppnas automatiskt över ett sparat projekt. Ofullständiga ogiltiga inmatningsvärden kan inte skrivas som ett giltigt återställningsprojekt.

## Arkitektur

- `src/model.rs`: projekt, portidentitet, anslutningsintegritet, rötter och vägar.
- `src/ports.rs`: typat, utbyggbart bibliotek med former, kontaktfamiljer och fysiska mått.
- `src/geometry.rs`: 3D-ytor, ortografisk kamera, rotation, koordinatomvandling och synlighetskontroll.
- `src/scene.rs`: native rendering av schematiska 3D-modeller och fysiska portar, markering och drag.
- `src/tree.rs`: hela skogens layout; extralänkar duplicerar inte enheter.
- `src/storage.rs`: versionerad JSON, validering, säker ersättning och återställning.
- `src/ui.rs`: egui/eframe, de två navigeringsvyerna, editor och ändringsskydd.
- `src/palette.rs`: alla portikoner och native dra-och-släpp.
- `src/port_label.rs`: dubbelklicksdialog för enkla utrustningsetiketter.

3D-renderaren är skriven i Rust och ritar projicerade polygoner genom egui:s OpenGL-backend. Det ger en liten fristående exe och en schematisk modell utan en andra grafikmotor. Portplockning kontrollerar dold geometri. Boxar och kilar kan kombineras; detta är inte ett CAD-program med booleska operationer eller fotorealistisk rendering. Inget Tauri, Electron, HTML, CSS, JavaScript eller WebView används.

## Bygga och verifiera

Installera Rust stable och Visual Studio Build Tools med C++-komponenterna på Windows:

```powershell
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --locked
```

Programmet finns i `target\release\CableManagement.exe`. Det statiska C-runtimet innebär att ingen separat Visual C++-runtime behöver distribueras. GitHub Actions kör Windowskontrollerna och publicerar endast exe:n som byggartefakt.

Från Linux med MinGW-w64:

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --release --locked --target x86_64-pc-windows-gnu
```

Konfigurationen i `.cargo/config.toml` länkar GNU-runtimet statiskt. `objdump -p` kan verifiera att exe:n enbart importerar Windows systembibliotek. För ett native starttest av grafiken på Linux krävs X11-bibliotek och en X-server; `cargo run` öppnar samma Rust-gränssnitt. Filvalet på Windows är en native systemdialog, på Linux en native sökvägsdialog.

Det levererade programmets `--self-test <rapportfil>` kontrollerar demokedjan och sparning/öppning inklusive Windows filersättning, utan att öppna ett GUI-fönster. Testet använder tillfälliga filer bredvid rapportfilen och rensar dem efteråt.

## Avgränsningar i v0.2.0

Schematisk box-/kilmodellering och ortografisk visning; ingen full CAD, bildimport eller automatisk USB-/nätverksinventering. Kontaktbiblioteket kan utökas i Rust-koden. Vyvalet som sparas är en av de sex fasta vyerna. Modell- och trädvy använder samma dokument och duplicerar inte anslutningsdata.

Källkod, tester och dokumentation hör till detta GitHub-repo. Leveransmappen i Google Drive innehåller endast `CableManagement.exe`; projektfiler och återställningsfiler skapas där användaren sparar dem eller i lokal appdata.

Befintliga projekt i formatversion 1 kan öppnas utan konvertering. Porternas och kablarnas identiteter bevaras. Dubbelklick på en port som redan går till en mappad nod visar den anslutningen och erbjuder navigering; den ersätts aldrig automatiskt med en textetikett.
