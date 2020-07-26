---
title: "Rustberry PI - LCD"
date: 2020-07-01T14:20:17+02:00
draft: true
authors: ["Maciej Flak"]
images: [
    "/images/rustypi/lcd/20200718_204650.jpg",
    "/images/rustypi/lcd/20200719_122243.jpg",
    "/images/rustypi/lcd/lcd_bb.svg",
    ]

featured_image: 'images/rustypi/lcd/20200718_204650.jpg'

tags: ["rust", "raspberry pi"]


Summary: '
Uczę się podłączać i z sukcesem wyświetlić napis na wyświetlaczu LCD 2x16 znaków opartym na kontrolerze HD44780 z rustem.
'

# series: ["Rustberry PI"]
---

Po zapaleniu [diody LED](https://flakm.github.io/posts/rustyled/) zawsze przychodzi apetyt na więcej. W moim zestawie peryferiów do maliny kolejnym elementem był wyświetlacz LCD 2x16 znaków zgodnym z [HD44780](https://en.wikipedia.org/wiki/Hitachi_HD44780_LCD_controller).
Pora na wyświetlenie rytualnego `Witaj świecie` na ekranie.

## Przygotowanie i podłączenie

Oprócz zakupów z [poprzedniego posta](https://flakm.github.io/posts/rustyled/) potrzebujemy:

1. wyświetlacza (koszt około 10zł)
2. dużo przewodów męsko-męskich
3. Listwa pinowa (16 pinów) + zestaw do lutowania
4. opcjonalnie potencjometr do sterowania kontrastem wyświetlacza.

W pierwszej kolejności lutujemy listwę pinową do naszego ekranu. Jako, że lutowałem po raz pierwszy i nie jestem dumny z wyników zamiast pozywać zdjęcia odsyłąm do [filmiku](https://m.youtube.com/watch?v=uzxw1yl1s_M).

W kolejnym etapie podłączamy ekran do naszej maliny zgodnie z tabelą:

<!-- todo dodać styling -->
{{<table "table my-table table table-dark">}}
| PI            | LCD           |
| :------------ | -------------:|
| GND           | VSS           |
| 5V            | VDD           |
| GND (potencj) | V0            |
| GPIO 22       | RS            |
| GND           | RW            |
| GPIO 5        | E             |
|               | D0 - D3       |
| GPIO 26       | D4            |
| GPIO 19       | D5            |
| GPIO 13       | D6            |
| GPIO 6        | D7            |
| 5V            | A             |
| GND           | K             |
{{</table>}}

I schematem:

{{< figure src="/images/rustypi/lcd/lcd_bb.svg" class="img-lg">}}

Mój pełen układ wygląda następująco:

{{< figure src="/images/rustypi/lcd/20200719_122243.jpg" class="img-sm">}}

zgodnie z zasadą szybkiego testowania czy wszystko poprawnie podłączyliśmy możemy na malinie wykonać następujący skrypt:


```bash
pip install Adafruit_CharLCD
cat <<EOF > lcd.py
from time import sleep
from Adafruit_CharLCD import Adafruit_CharLCD

lcd = Adafruit_CharLCD(rs=22, en=5, d4=26,d5=19,d6=13,d7=6,cols=16,lines=2)

lcd.clear()

lcd.message('Hello\n world!')
sleep(3)
lcd.clear()

EOF
python lcd.py
```

## Dobra, dobra ale co właściwie się stało? 

Zgodnie z dokumentacją kontrolera pierwszym krokiem który trzeba wykonać jest inicjalizacja, czyli ustawienie sposobu działania i wybranie trybu działania kontrolera. Dostępne stany to: 

1. 8 bitowy
2. 4 bitowy - oczekiwanie na pierwsze 4 bity
3. 4 bitowy - oczekiwanie na drugie 4 bity

W pierwszym trybie wykorzystujemy 8 pinów GPIO do przesyłu danych i każdy z pinów podłączony jest do jednego wyjścia wyświetlacza, w drugim trybie wykorzystujemy jedynie 4 i w pierwszej kolejności wysyłamy 4 bardziej znaczące bity, pulsujemy stanem wyjścia `Enable` (tu przechodziny do stanu 3) i wysyłąmy kolejne, mniej znaczące 4 bity i pulsujemy stanem wyjścia `Enable`. Pulsowanie oznacza zmianę Ze stanu wysokiego do niskiego.

W jaki sposób można zamodelować drobną bibliotekę, która będzie wykonywała to zadanie?
W pierwszej kolejności należy utworzyć nowy projekt:

```bash
# tworzymy nowy projekt w katalogu lcd
cargo new lcd --bin
cd lcd

rustup target add armv7-unknown-linux-gnueabihf
# install arm linker
sudo apt-get install -qq gcc-arm-linux-gnueabihf
mkdir .cargo
cat <<EOF > .cargo/config
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
EOF
```

Oraz modyfikujemy cargo.toml poprzez dodanie dwóch zależności:

```toml
rppal = "0.11.3"
anyhow = "1.0"
```

Aby utrzymać minimalny porządek w kodzie możemy utworzyć dodatkowy plik `src/lib.rs`, w którym umieszczamy kod naszej biblioteki do komunikacji z ekranem:

```rust
use anyhow::Result;
use rppal::gpio::Gpio;
use rppal::gpio::OutputPin;
use std::{thread, time};


pub struct Lcd {
    /// cztery piny do przesyłu danych (bit 4-7)
    pub data: [OutputPin; 4],

    /// pin wyboru rejestru 
    pub rs: OutputPin,

    /// pin enable
    pub en: OutputPin,
}

const ROW_OFFSET: [u8; 2] = [0x00, 0x40];

// commands: 
const LCD_SETDDRAMADDR: u8 = 0x80; 

impl Lcd {
    pub fn new() -> Result<Lcd> {
        let rs = Gpio::new()?.get(22)?.into_output();

        let enable = Gpio::new()?.get(5)?.into_output();

        let d4 = Gpio::new()?.get(26)?.into_output();
        let d5 = Gpio::new()?.get(19)?.into_output();
        let d6 = Gpio::new()?.get(13)?.into_output();
        let d7 = Gpio::new()?.get(6)?.into_output();

        Ok(Lcd {
            data: [d4, d5, d6, d7],
            rs: rs,
            en: enable,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        // initialize 
        self.write(0x33, false)?;
        self.write(0x32, false)?;

        
        self.write(0x08 | 0x04 , false)?; // LCD_DISPLAYON LCD_DISPLAYCONTROL
        self.write(0x20 |  0x00  | 0x08, false)?; // LCD_4BITMODE 2 lines function set
        self.write(0x04 | 0x02, false)?; // LCD_ENTRYLEFT
        self.clear()?;
        Ok(())
    }

    pub fn clear(&mut self)-> Result<()> {
        self.write(0x01, false)?;  // command to clear display LCD_CLEARDISPLAY
        thread::sleep(time::Duration::new(0, 1570000));  // 1,57 ms sleep, clearing the display takes a long time
        Ok(())
    }

    pub fn message(&mut self, text: String) -> Result<()> {
        self.clear()?;
        let mut line = 0;

        for char in text.chars() {
            if char == '\n' {
                line += 1;
                self.set_cursor(0, line)?
            } else {
                // todo perform some check of casting
                self.write(char as u8, true)?
            }
        }
        Ok(())
    }

    pub fn set_cursor(&mut self, col: u8, row: u8) -> Result<()> {
        let mut actual_row = row;
        // Clamp row to the last row of the display.
        if row > 2 { // max row
            actual_row = 1
        }
        self.write(LCD_SETDDRAMADDR | (col + ROW_OFFSET[actual_row as usize]), false)?;
        Ok(())
    }

    fn pulse_enable(&mut self) {
        let write_pin = |pin: &mut OutputPin, enabled: bool| {
            if enabled {
                pin.set_high()
            } else {
                pin.set_low()
            }
        };

        // Breathing time
        thread::sleep(time::Duration::new(0, 450));

        // enable pulse must be > 450ns
        write_pin(&mut self.en, false);
        thread::sleep(time::Duration::new(0, 1000));

        // enable pulse must be > 450ns
        write_pin(&mut self.en, true);
        thread::sleep(time::Duration::new(0, 450));

        // commands need 37us to settle
        write_pin(&mut self.en, false);
        thread::sleep(time::Duration::new(0, 37 * 1000));
    }

    pub fn write(&mut self, value: u8, char_mode: bool) -> Result<()> {
        let write_pin = |pin: &mut OutputPin, enabled: bool| {
            if enabled {
                pin.set_high()
            } else {
                pin.set_low()
            }
        };

        write_pin(&mut self.en, false);
        write_pin(&mut self.rs, char_mode);

        write_pin(&mut self.data[0], value & 0b0001_0000u8 > 0);
        write_pin(&mut self.data[1], value & 0b0010_0000u8 > 0);
        write_pin(&mut self.data[2], value & 0b0100_0000u8 > 0);
        write_pin(&mut self.data[3], value & 0b1000_0000u8 > 0);

        self.pulse_enable();

        write_pin(&mut self.data[0], value & 0b0000_0001u8 > 0);
        write_pin(&mut self.data[1], value & 0b0000_0010u8 > 0);
        write_pin(&mut self.data[2], value & 0b0000_0100u8 > 0);
        write_pin(&mut self.data[3], value & 0b0000_1000u8 > 0);

        self.pulse_enable();

        Ok(())
    }
}
```

Pierwszym krokiem jest utworzenie struct `Lcd`:

```rust
pub struct Lcd {
    /// cztery piny do przesyłu danych (bit 4-7)
    pub data: [OutputPin; 4],

    /// pin wyboru rejestru 
    pub rs: OutputPin,

    /// pin enable
    pub en: OutputPin,
}
```

Zawiera referencje do wszystkich pinów, które są nam potrzebne do wykonywania operacji w trybie 4-bitowym. Aby dodać metody należy zdeklarować je w bloku `impl` jak w przypadku metody konstruującej `new`:

```rust
impl Lcd {
    pub fn new() -> Result<Lcd> {
        let rs = Gpio::new()?.get(22)?.into_output();

        let enable = Gpio::new()?.get(5)?.into_output();

        let d4 = Gpio::new()?.get(26)?.into_output();
        let d5 = Gpio::new()?.get(19)?.into_output();
        let d6 = Gpio::new()?.get(13)?.into_output();
        let d7 = Gpio::new()?.get(6)?.into_output();

        Ok(Lcd {
            data: [d4, d5, d6, d7],
            rs: rs,
            en: enable,
        })
    }
    /// rest skipped for now
```

Wykorzystujemy bibliotekę `rppal` i konstruujemy zestaw pinów wyjściowych:

```rust
let rs = Gpio::new()?.get(22)?.into_output();
```

W kodzie pojawia się znak `?`, jest to operator, który pomaga przy obsłudze błędów.
Użycie `?` oznacza, że nasz kod ma w tym miejscu zwrócić sterowanie wyżej, przekazując błąd jeżeli zaistniał lub odpakować poprawny wynik. Kompilator nie pozwoli nam popełnić błędu i zmusza nas do obsłużenia wszystkich możliwości. 

Mamy już gotową instancję `Lcd` zawierającą wszystkie piny potrzebne do komunikacji. Teraz należy zainicjować nasz wyświetlacz:

```rust
    pub fn init(&mut self) -> Result<()> {
        // checkout Mode selection in  https://en.wikipedia.org/wiki/Hitachi_HD44780_LCD_controller
        // this will set controller to 4-bit mode no matter the entering state
        self.write(0x33, false)?;
        self.write(0x32, false)?;

        thread::sleep(time::Duration::new(0, 4 * 1000)); // wait?

        // Turn display on and display control
        self.write(0x08 | 0x04, false)?;
        thread::sleep(time::Duration::new(0, 4 * 1000)); // wait?

        // Set Entry left mode
        self.write(0x06, false)?;
        thread::sleep(time::Duration::new(0, 4 * 1000)); // wait?

        self.clear()?;
        Ok(())
    }
```

W powyższym fragmencie metoda `init` przyjmuje jeden parametr `&mut self`. Oznacza to, ze przyjmuje mutowalną referencję do samego siebie. Mutowalna referencja oznacza, że mamy możliwość modyfikowania wartości bez przejmowania własności.

Pierwsze dwie wartości `0x33` i `0x32` zagwarantują, że dalsza część programu będzie działała w trybie 4 bitowym bez znaczenia jaki stan początkowy zastał program. W dalszej części metody ustawiane są pozostałe wymagane parametry pracy kontrolera.

Wywołanie `self.clear()` czyści bufory kontrolera. W przypadku wcześniejszego stanu spowoduje, to że po wykonaniu metody `init` ekran będzie czysty.

Kluczowym fragmentem naszego programu jest metoda `write`:

```rust
    pub fn write(&mut self, value: u8, char_mode: bool) -> Result<()> {
        let write_pin = |pin: &mut OutputPin, enabled: bool| {
            if enabled {
                pin.set_high()
            } else {
                pin.set_low()
            }
        };

        write_pin(&mut self.en, false);
        write_pin(&mut self.rs, char_mode);

        write_pin(&mut self.data[0], value & 0b0001_0000u8 > 0);
        write_pin(&mut self.data[1], value & 0b0010_0000u8 > 0);
        write_pin(&mut self.data[2], value & 0b0100_0000u8 > 0);
        write_pin(&mut self.data[3], value & 0b1000_0000u8 > 0);

        self.pulse_enable();

        write_pin(&mut self.data[0], value & 0b0000_0001u8 > 0);
        write_pin(&mut self.data[1], value & 0b0000_0010u8 > 0);
        write_pin(&mut self.data[2], value & 0b0000_0100u8 > 0);
        write_pin(&mut self.data[3], value & 0b0000_1000u8 > 0);

        self.pulse_enable();

        Ok(())
    }
```

<!-- https://www.microchip.com/forums/m786498.aspx  -->
<!-- https://exploreembedded.com/wiki/Interfacing_LCD_in_4-bit_mode_with_8051 -->

W trybie 4 bitowym używane są piny kontrolne i 4 równoległe linie danych.

1. Wybór rejestru (**register select RS**) pozwala sterować do którego rejestru mają być zapisywane sygnały. Kontroler posiada dwa rejestry - danych (stan wysoki) i poleceń - (stan niski)

2. Zapis/odczyt (**read/write RW**) dla zapisu powinien być ustawiony na stan niski. Dlatego w naszym układzie podłączony jest do pinu GND

3. Włącz (**Enable EN**) steruje wzbudzeniem działania kontrolera. Zmiana ze stanu wysoki na niski jest wymagana aby wywołać zadeklarowane działanie

4. Piny danych (**D4-D7**) pozwalające na przesyłanie znaków do wyświetlenia lub komend

W takim razie w jaki sposób zostało zaimplementowane "pulsowanie"?

```rust
    fn pulse_enable(&mut self) {
        let write_pin = |pin: &mut OutputPin, enabled: bool| {
            if enabled {
                pin.set_high()
            } else {
                pin.set_low()
            }
        };

        // Breathing time
        thread::sleep(time::Duration::new(0, 450));

        // enable pulse must be > 450ns
        write_pin(&mut self.en, false);
        thread::sleep(time::Duration::new(0, 1000));

        // enable pulse must be > 450ns
        write_pin(&mut self.en, true);
        thread::sleep(time::Duration::new(0, 450));

        // commands need 37us to settle
        write_pin(&mut self.en, false);
        thread::sleep(time::Duration::new(0, 37 * 1000));
    }
```

Deklarujemy lokalnego closurka `write_pin`, który zmienia stan pojedynczego pina.
Następnie defensywnie zmieniamy stan pina `EN` z na niski -> wysoki -> niski, żeby mieć pewność, że bez warunku jaki był wcześniejszy stan doszło do "zapulsowania".
Oczekiwanie wykonywane przez `thread::sleep` jest wymagane przez kontroler zgodnie z wymaganiami w dokumentacji.

Wygląda na to, że wszystkie komponenty oprócz pisania tekstu do samego kontrolera są już zaimplementowane, Zatem na czym polega pisanie tekstu?

```rust
    pub fn message(&mut self, text: String) -> Result<()> {
        self.clear()?;
        let mut line = 0;

        for char in text.chars() {
            if char == '\n' {
                line += 1;
                self.set_cursor(0, line)?
            } else {
                // todo perform some check of casting
                self.write(char as u8, true)?
            }
        }
        Ok(())
    }

    pub fn set_cursor(&mut self, col: u8, row: u8) -> Result<()> {
        let mut actual_row = row;
        // Clamp row to the last row of the display.
        if row > 2 { // max row
            actual_row = 1
        }
        self.write(LCD_SETDDRAMADDR | (col + ROW_OFFSET[actual_row as usize]), false)?;
        Ok(())
    }
```

Ponownie defensywnie czyścimy na początku bufory controllera. a następnie iterujemy po wszystkich literach ciągu. Jeżeli natrafimy na znak nowej lini `\n` zmieniamy kursor na odpowiednią linię w przeciwnym przypadku zapisujemy odpowiednie znaki.
W powyższym kodzie znajduje się dużo możliwości poprawy:

1. co jeżeli wejście ma w linijce więcej niż 16 znaków?
2. co jeżeli lini jest więcej niż wyświetlacz posiada?
3. co jeżeli znak jest spoza zakresu 0-255 (typ na który castujemy to u8 czyli 8 bitów)?

W przypadku 1 zachowanie ekranu jest niezdefiniowane. W 2 przypadku instrukcja warunkowa spowoduje, że 3 i kolejne linie będą nadpisywały drugą linię.
W 3 przypadku problem może być większy bo zgodnie z zasadami języka dojedzie do błędnego wyniku:

```rust
    #[test]
    fn cast() {
        let hmm = 256;
        assert_eq!(hmm, 0)
    }
```

Aby ograniczyć te problemy możemy dodać walidację wejścia przekazywanego przez użytkownika.
Zmieniony kod wygląda następująco:

```rust
fn process_msg(text: &str) -> Result<Vec<u8>> {
        if text.lines().count() > 2 {
            return Err(anyhow!(
                "Invalid line count. We have only 2 lines on screen"
            ));
        }

        for (i, line) in text.lines().map(|l| l.replace("\n", "")).enumerate() {
            if line.chars().count() > 16 {
                return Err(anyhow!(
                    format!("Line number {} has more then 16 allowed characters", i)
                ));
            }
        }

        let mut result = vec![];

        for char in text.chars() {
            result.push(u8::try_from(char as u32)?);
        }

        Ok(result)
    }

    pub fn message(&mut self, text: String) -> Result<()> {
        self.clear()?;
        let mut line = 0;

        // let characters = text.chars();
        let characters = Lcd::process_msg(&text)?;

        for char in characters {
            if char == '\n' as u8 {
                line += 1;
                self.set_cursor(0, line)?
            } else {
                // todo perform some check of casting
                self.write(char as u8, true)?
            }
        }
        Ok(())
    }
```

W nowej prywatnej funkcji `process_msg` przetwarzamy otrzymany ciąg znaków i zwracamy wektor znaków gotowych do zapisu o typie `Vec<u8>`. Jak to osiągneliśmy? W pierwszej kolejności sprawdzamy czy liczba lini jest taka jak się spodziewamy:

```rust
if text.lines().count() > 2 {
    return Err(anyhow!(
        "Invalid line count. We have only 2 lines on screen"
    ));
}
```

Jeżeli jest większa niż spodziewane 2 to zwracamy błąd używając wariantu enuma `Result` sugerującego błąd: `Err`.
Używamy też rewelacyjnej biblioteki `anyhow` aby nie tworzyć zbędnych typów błędów. W większej bibliotece możliwe, że miałoby to większe znaczenie i można by użyć alternatywnej `thiserror`.

W kolejnym kroku iterujemy po każdej linijce i sprawdzamy, czy każda linijka posiada maksymalnie 16 znaków:

```rust
 for (i, line) in text.lines().map(|l| l.replace("\n", "")).enumerate() {
            if line.chars().count() > 16 {
                return Err(anyhow!(
                    format!("Line number {} has more then 16 allowed characters", i)
                ));
            }
        }
```

Ostatecznie przepisujemy znaki do wektora gotowego do zapisu na naszym ekranie walidując przy okazji czy wskazane znaki znajdują się w odpowiednim zakresie 0-255.

```rust
        let mut result = vec![];

        for char in text.chars() {
            result.push(u8::try_from(char as u32)?);
        }

        Ok(result)
```

Aby mieć pewność, że nasze zmiany faktycznie działają możemy napisać kilka testów:

```rust
#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn check_our_validation(){
        assert!( Lcd::process_msg("hello \n world").is_ok());
        assert!( Lcd::process_msg("hello \n world\n").is_ok());
        assert!( Lcd::process_msg("hello \n world\n third line\n").is_err() );
        assert!( Lcd::process_msg("hello this is a long sentence\n world").is_err() );
        assert!( Lcd::process_msg("ą").is_err());
    }
}
```

Po uruchomieniu testu używając polecenia `cargo test --lib` widzimy, że wszystko jest ok:

```shell
$ cargo test --lib
    Finished test [unoptimized + debuginfo] target(s) in 0.00s
     Running target/debug/deps/lcd-b10bed0c8c7b28a9

running 1 test
test tests::check_our_validation ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

```

Pozostaje szybki test wykonania na malinie. W pliku `src/main.rs` zapisujemy:

```rust
use anyhow::Result;

use lcd::Lcd;
use std::{thread, time};

fn main() -> Result<()> {

    let mut pins = Lcd::new()?;

    pins.init()?;
    pins.message(String::from("witaj swiecie \n   rust :)"))?;
    thread::sleep(time::Duration::new(2, 0));

    pins.message(String::from("witaj swiecie \n   rust ;)"))?;
    thread::sleep(time::Duration::new(2, 0));

    pins.clear()?;

    Ok(())
}
```

Całość możemy skompilować używając polecenia `cargo build --target armv7-unknown-linux-gnueabihf` i przesłać na malinę używając scp.

## Podsumowanie

Jesteśmy w stanie bez problemu sterować tekstem na naszym wyświetlaczu.
Dodatkowo mamy bibliotekę, która wykonuje podstawową walidację wejścia.
Podobnie jak w scali istnieją typy (w scali zamiast `Result` jest to `Try`) które sygnalizują, że dany fragment kodu może się nie udać i wymusza to na nas dbałość o zaopiekowanie każdej możliwości (w scali dopiero przy pattern matchingu).
Aby rozwinąć dalej projekt należało by umożliwić parametryzowanie działania (które piny podłączamy czy w jakim trybie działamy) przykładowo przy użyciu crate [config](https://crates.io/crates/config).
