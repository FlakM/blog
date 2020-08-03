---
title: "Rustberry PI - DHT11"
date: 2020-08-02T13:12:54+02:00
draft: false

authors: ["Maciej Flak"]
images: [
    "/images/rustypi/dht11/20200802_131543.jpg",
    ]

featured_image: 'images/rustypi/dht11/20200802_131543.jpg'

tags: ["rust", "raspberry pi"]

Summary: '
Uczę się obsługiwać czujnik wilgoci i temperatury DHT11 z użyciem rust i prezentuję wyników na ekranie lcd używając biblioteki zbudowanej w poprzednim wpisie
'
series: ["raspberry pi"]
---

{{< rusty-github >}}

W poprzednim wpisie o [wyświetlaczu LCD](https://flakm.github.io/posts/rustylcd/) uczyłem się jak przy wykorzystaniu maliny, rusta i prostego wyświetlacza zaprezentować dane użytkownikowi.
Kolejnym elementem w moim zestawie startowym jest czujnik wilgotności [DHT11](https://www.waveshare.com/wiki/DHT11_Temperature-Humidity_Sensor).

Czujnik DHT11 zgodnie z [dokumentacją](https://www.waveshare.com/w/upload/c/c7/DHT11_datasheet.pdf) jest w stanie mierzyć relatywną wilgotność z dokładnością +/-5% a temperatury +/-2°C.
Zupełnie wystarczająco na potrzeby domowego projektu. Do komunikacji z maliną wykorzystuje pojedynczy pin GPIO. Sterując czujnikiem należy wysyłać i odbierać sygnały w określonej sekwencji zgodnej z protokołem - inaczej czujnik nie będzie wysyłał poprawnych pomiarów.

## Budowa projektu

W pierwszej kolejności w `lib.rs` tworzę obiekty domenowe:

```rust
pub struct DHT11 {
    pin: IoPin,
}

#[derive(Debug)]
pub struct Readings {
    pub temperature: f64,
    pub humidity: f64,
}
```

Sygnał odebrany od czujnika składa się z 40 bitów.

1. Pierwszy bajt zawiera część odczytu wilgotności przed przecinkiem
2. Drugi bajt zawiera część odczytu wilgotności po przecinku
3. Trzeci bajt zawiera cześć odczytu temperatury przed przecinkiem
4. Czwarty bajt zawiera część odczytu temperatury po przecinku
5. Piąty bajt zawiera sumę kontrolną - sumę poprzednich bajtów

Dlatego struct `Readings` zawiera dwa pola - jedno zawierające temperraturę, drugie wilgotność.
Jako, że czujnik DHT11 do komunikacji używa pojedynczego pina który jest zarówno wejściem i wyjściem to struct `DHT11` zawiera jedno pole. Oprócz tego aby umożliwić czytelny wydruk (czasem wartości mogą mieć podobne wartości) należy zaimplementować traita `fmt::Display`:

```rust
impl fmt::Display for Readings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Readings({:.2}%, {:.2}C)",
            self.humidity, self.temperature
        )
    }
}
```

Pora rozpocząć implementację odczytu wartości naszego czujnika!

```rust

impl DHT11 {
    pub fn new(gpio_num: u8) -> Result<DHT11> {
        Ok(DHT11 {
            pin: Gpio::new()?.get(gpio_num)?.into_io(Mode::Output),
        })
    }

    /// initialize
    fn init_sequence(&mut self) -> () {
        // step 2 of documentation
        self.pin.set_mode(Mode::Output);
        self.pin.set_low();
        delay_ms(18);
        self.pin.set_high();
        delay_us(50)
    }

    pub fn read(&mut self) -> Result<Readings> {
        self.init_sequence();

        let mut bytes = [0u8; 5];

        {
            // step 3 of documentation
            self.pin.set_mode(Mode::Input);
            wait_level(&mut self.pin, Level::Low)?;
            wait_level(&mut self.pin, Level::High)?;
            wait_level(&mut self.pin, Level::Low)?;

            // step 4 of documentation
            for b in bytes.iter_mut() {
                for _ in 0..8 {
                    *b <<= 1;
                    wait_level(&mut self.pin, Level::High)?;
                    let dur = wait_level(&mut self.pin, Level::Low)?;
                    if dur > 26 {
                        *b |= 1;
                    }
                }
            }
        }

        let sum: u16 = bytes.iter().take(4).map(|b| *b as u16).sum();
        if bytes[4] as u16 == sum & 0x00FF {
            Ok(Readings {
                temperature: bytes[2] as f64 + (bytes[3] as f64 / 10.0),
                humidity: bytes[0] as f64 + (bytes[1] as f64 / 10.0),
            })
        } else {
            Err(anyhow!("Check sum!"))
        }
    }
}

fn wait_level(pin: &mut IoPin, level: Level) -> Result<u8> {
    for i in 0u8..255 {
        if pin.read() == level {
            return Ok(i);
        }
        delay_us(1);
    }
    Err(anyhow!("Timeout!"))
}

```

Dodajemy funkcję `new` która umożliwia na wskazanie numeru pin, do którego będzie podłączona szyna danych w naszym czujniku.

```rust
    pub fn new(gpio_num: u8) -> Result<DHT11> {
        Ok(DHT11 {
            pin: Gpio::new()?.get(gpio_num)?.into_io(Mode::Output),
        })
    }
```

Zgodnie z dokumentacją czujnika, przed każdym odczytem należy wykonać inicjalizację, czyli ustawić pin w stan niski na 18 ms i zmienić na stan wysoki:

```rust
    fn init_sequence(&mut self) -> () {
        // step 2 of documentation
        self.pin.set_mode(Mode::Output);
        self.pin.set_low();
        delay_ms(18);
        self.pin.set_high();
        delay_us(50)
    }
```

W dalszej kolejności musimy przejść w tryb wejściowy i odebrać 40 bitów sygnału.
Zadanie to wykonuje metoda `read` przyjmująca jako argument mutowalną referencję do samego siebie.
W tym momencie musimy oczekiwać w trybie wejściowym na sekwencję sygnałów:

```rust
    // step 3 of documentation
    self.pin.set_mode(Mode::Input);
    wait_level(&mut self.pin, Level::Low)?;
    wait_level(&mut self.pin, Level::High)?;
    wait_level(&mut self.pin, Level::Low)?;
```

Teraz możemy zainicjować tablicę spodziewanych 5 bajtów i przypisać im odpowiednie wartości:

```rust
let mut bytes = [0u8; 5];
// step 4 of documentation
for b in bytes.iter_mut() {
    for _ in 0..8 {
        *b <<= 1;
        wait_level(&mut self.pin, Level::High)?;
        let dur = wait_level(&mut self.pin, Level::Low)?;
        if dur > 26 {
            *b |= 1;
        }
    }
}
```

Pozostaje sprawdzić czy suma kontrolna znajdująca się w ostatnim bajcie jest zgodna z odczytami.

```rust
let sum: u16 = bytes.iter().take(4).map(|b| *b as u16).sum();
if bytes[4] as u16 == sum & 0x00FF {
    Ok(Readings {
        temperature: bytes[2] as f64 + (bytes[3] as f64 / 10.0),
        humidity: bytes[0] as f64 + (bytes[1] as f64 / 10.0),
    })
} else {
    Err(anyhow!("Check sum!"))
}
```

Pierwszy bajt zawiera pełnoliczbową część odczytu wilgotności, drugi część dziesiętną. Analogicznie w przypadku bajtów 3 i 4 jedynie dla temperatury.
Suma kontrolna liczona jest jako suma kolejnych bajtów:

```rust
bytes.iter().take(4).map(|b| *b as u16).sum()
```

## Złożenie kodu w całość

W celu prezentacji otrzymanych odczytów możemy użyć układu z poprzedniego wpisu i wyświetlić je na wyświetlaczu LCD. W tym celu najpierw musimy dodać zależność do modułu z biblioteką zawierającą kod z poprzedniego rozdziału. Zakładając taką samą strukturę kodu jak w repozytorium [rustberry](https://github.com/FlakM/rustberry) starczy dodać jedną linijkę do pliku cargo.toml w bloku zależności:

```toml
lcd = { path = "../lcd" }
```

A następnie w pliku `main.rs` możemy napisać:

```rust
use anyhow::Result;
use dht11::DHT11;
use lcd::Lcd;

use std::{thread, time::Duration};
fn main()-> Result<()> {
    let mut sensor = DHT11::new(21)?;
    let mut lcd = Lcd::new()?;
    lcd.init()?;

    loop {

        let result = sensor.read();

        match result {
            Ok(readings) => {
                println!("{}", readings);
                let msg = format!("Temp:      {:.1}C\nHumid:     {:.1}%", readings.temperature, readings.humidity);
                lcd.message(msg)?

            },

            Err(err) => eprintln!("{}",err)
        }

        
        thread::sleep(Duration::from_secs(3));
    }

}
```

Jeżeli wykonaliśmy poprawnie podłączenie czujnika i ekranu, to dostaniemy w efekcie odczyty wyświetlane na naszym ekranie!

## Podsumowanie

Nauczyłem się obsługiwać czujnik wilgotności i temperatury DHT11 przy wykorzystaniu pinów GPIO w trybie wejścia/wyjścia.
Przy okazji nauczyłem się używać własnych bibliotek jako podmoduły projektu.
Okazuje się, że łączenie wielu projektów przy wykorzystaniu `cargo` jest bardzo przyjemne i nie różni się niczym względem dodania zależności do crate z crate.rs.
