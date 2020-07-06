---
title: "Zardzewiała dioda"
date: 2020-07-01T14:13:56+02:00
draft: true
authors: ["flakm"]
images: ["/images/yannick-pipke-GtcA8mw0t1U-unsplash.jpg", "images/rystypi/leds/taśma.jpg","images/rystypi/leds/układ.jpg" ]
---


Przez ostatnie kilka lat zajmowałem się programowaniem aplikacji biznesowych.
Ze względu na swoją historię nauki informatyki nigdy nie miałęm okazji uczestniczyć w lekcjach elektroniki.
Od zawsze używałem bardzo wysokich warstw abstrakcji dalego od samego sprzętu.
Względnie niedawno odkryta fascynacja językiem rust i rewelacyjna społeczność spowodowała, że podjąłem się próby nauki na własną rękę.


Możliwość okiełznania ruchu elektronów okazała się bardzo fascynująca.
Jest coś niesamowitego w programowaniu tak materialnych rzeczy jak ukłądy elektroniczne.


Swoją przygodę zacząłem od nadrobienia podstawowych lekcji elektroniki.
Bardzo polecam dowolną pozycję o elektronice. Wiedza na temat tego co się dzieje i jak działa prąd zwiększy bezpieczeństwo (nasze i delikatnych układów scalonych) oraz zapewni dużo większą satysfakcję z całego procesu.


## Wymagania początkowe

Do zbudowania układu opisanego w tym wpisie konieczne jest posiadanie bardzo niewielu elementów: 

- Raspberry pi (właściwie dowolny model, ja posiadam 2B)
- karta pamięci zgodna z wymaganiami maliny
- karta rozszerzeń GPIO
- Taśma 40-pin do karty GPIO
- płytka stykowa
- przewody męsko męskie
- dioda led
- rezystor o odpowiedniej wartości w moim przypakdu 4,7kohm

W pierwszej kolejności należy zadbać o to, żeby na karcie pamięci pojawił się zainstalowany aktualny system.
Można podejść do tego zgodnie z (instrukcją producenta)[https://www.raspberrypi.org/documentation/installation/installing-images/].
Podstawowe dane logowania to `pi` i hasło `raspberry`. Za pierwszym razem musimy się zalogować przy użyciu wyjścia hdmi i klawiatury fizycznej. 
Aby umożliwić wygodną dalszą pracę w tym miejscu warto zadbać o możliwość zdalnego połączenia poprzez włączenie usługi ssh na malinie.
Wyczerpująca instrukcja jak przejść przez ten proces dostępna jest (tutaj)[https://www.raspberrypi.org/documentation/remote-access/ssh/].

Dodatkowo zakładam, że kod napisany w rust będę uruchamiał na swoim laptopie z linuxem.
Możliwe jest wykonanie tego samego procesu używając dowolnego systemu a nawet na samej malinie.
Wybieram model pracy z kompilacją na swoim laptopie ze względu na czas kompilacji i obecność wszystkich wymaganych narzędzi.

Wymagane narzędzia do pracy z kodem w rust:

- narzędzia do kompilacji rust, polecam instalację zgodnie z https://rustup.rs/
- dowolny edytor tekstu, polecam VScode z wtyczką Rust Analyzer
- ssh i scp do połączenia zdalnego i przesłania skompilowanego projektu

## Czemu rust i malina

Prosta odpowiedź brzmi: bo czemu nie? Natomiast dłuższa to:

Już posiadałem zakurzoną malinę z przed kilku lat w szafie. Dodatkowo jest to względnie łatwo dostępny komputer (można nabyć nową w cenie około 200 złotych).
Ogromnym atutem jest też mnogość dostępnych artykułów na temat gotowych projektów.
Każdy z nowych układów buduję na podstawię projektów już gotowych (kod zazwyczaj w pythonie).

Z kolejnej strony język programowania wybrałem ze względu na potencjał użycia go w projektach na mniej potężnych płytkach do bardziej wyspecjalizowanych zadań.
Bez sprzeczności jedną z największych zalet rust jest dbanie o bezpieczeństwo kodu (kompilator bardzo się stara do granic możliwości ograniczyć moją niedokładność).
Kolejnym wielkim atutem jest zestaw narzędzi do budowania, `cargo` jest niesamowicie pomocne na każdym etapie projektu. 

## Budowa oraz testowanie układu

Do maliny podłączamy taśmę gpio jak na zdjęciu:



{{< figure src="/images/rystypi/leds/taśma.jpg" class="img-lg">}}


A następnie taśmę do płytki rozszerzeń i zamontować ją na płytce stykowej.
Pełen układ jest widoczny poniżej:

{{< figure src="/images/rystypi/leds/układ.jpg" class="img-lg">}}


- **Pin GPIO23** jest połączony czerwonym przewodem z szyną dodatnią
- **PIN GND** jest połączony z szyną ujemną
- dioda LED (krótsza nóżka powinna być połączona z ujemną szyną)
- rezystor o wartości 4,7Kohm 
- przewód zamykający obwód

Aby przetestować układ możemy uruchomić skrypt (na malinie):


```bash
cat <<EOF > led.py
from gpiozero import LED

from time import sleep

led = LED(23)

while True:
   led.on()
   sleep(1)
   led.off()
   sleep(1)
EOF
python led.py
```

Dioda powinna zacząć migać z przerwami 1 sekundy. 
Aby przerwać program należy nacisnąć klawisze CTRL+C.


## Budowa programu w rust

Aby rozpocząć projekt na maszynie na której chcemy budować kod należy stworzyć nowy projekt:

```bash
# tworzymy nowy projekt w katalogu rusty_led
cargo new rusty_led --bin
cd rusty_led

# instalacja rustup powoduje zainstalowanie bibliotek dla naszego środowiska
# aby zainstalować biblioteki dla raspberry pi należy wykonać polecenie:
rustup target add armv7-unknown-linux-gnueabihf
# install arm linker
# todo sprawdzić czy to jest potrzebne
sudo apt-get install -qq gcc-arm-linux-gnueabihf
tree .
```

Musimy dodać w `~/.cargo/config` następującą treść:

```bash
mkdir .cargo
cat <<EOF > .cargo/config
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
EOF
```

Następnie możemy uruchomić kompilację:

```bash
cargo build --target armv7-unknown-linux-gnueabihf
```

Gotowy plik wykonywalny jest dostępny w pliku `target/armv7-unknown-linux-gnueabihf/debug/rusty_led`.
Aby go wykonać na malinie musimy go wysłać na malinę.
Zakładając, że znamy adres ip maliny (w moim przypadku 192.168.8.103) oraz, że skonfigurowaliśmy odpowiednie klucze prywatne i publiczne możemy go przesłać używając polecenia:

```bash
# przesyłamy plik
scp target/armv7-unknown-linux-gnueabihf/debug/rusty_led pi@192.168.8.113:~/
# uruchamiamy skompilowany plik wykonywalny
ssh pi@192.168.8.113 './rusty_led'

#   output:
#   Hello, world!
```

Teraz w dalszym kroku należy dodać zależność do biblioteki która umożliwi nam używanie GPIO.
Zeby to zrobić starczy dopisać do pliku `Cargo.toml` w sekcji `[dependencies]`:


```toml
rppal = "0.11.3"
```

A treść pliku main zamienić 

```rust
use std::error::Error;
use std::thread;
use std::time::Duration;

use rppal::gpio::Gpio;
use rppal::system::DeviceInfo;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Działam na {}.", DeviceInfo::new()?.model());

    let mut pin = Gpio::new()?.get(23)?.into_output();
    pin.set_reset_on_drop(true);

    loop {
        pin.set_high();
        thread::sleep(Duration::from_millis(500));
        pin.set_low();
        thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}
```

Po skompilowaniu, przesłaniu i wykonaniu programu na malinie osiągamy analogiczny efekt do kodu z pythona.
Jest jeden problem: zachowanie naszego układu elektronicznego po wyjściu z programu jest niedeterministyczne. 



def _devices_shutdown():
    if Device.pin_factory is not None:
        with Device.pin_factory._res_lock:
            reserved_devices = {
                dev
                for ref_list in Device.pin_factory._reservations.values()
                for ref in ref_list
                for dev in (ref(),)
                if dev is not None
            }
        for dev in reserved_devices:
            dev.close()
        Device.pin_factory.close()
        Device.pin_factory = None


/usr/lib/python2.7/dist-packages/gpiozero/devices.py
https://stackoverflow.com/questions/53618198/using-gpiozero-on-raspberry-pi-to-control-pins-but-output-pins-are-reset-up

https://github.com/gpiozero/gpiozero/issues/707