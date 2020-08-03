---
title: "Rustberry PI - LED"
date: 2020-07-01T14:13:56+02:00
draft: false
authors: ["Maciej Flak"]
images: [
    "/images/yannick-pipke-GtcA8mw0t1U-unsplash.jpg", 
    "images/rustypi/leds/taśma.jpg",
    "images/rustypi/leds/układ.jpg",
    "images/rustypi/leds/led_bb.png",
    "images/rustypi/leds/it-crowd-gif-4.gif"
    
    ]
featured_image: '/images/yannick-pipke-GtcA8mw0t1U-unsplash.jpg'

tags: ["rust", "raspberry pi"]

Summary: '
Nie krótko i nie na temat o tym jak uczę się czym jest GPIO, jak się nim steruje na oraz w efekcie - jak zapalać diodę LED przy wykorzystaniu GPIO i rusta 🦀
'

series: ["raspberry pi"]
---

{{< rusty-github >}}

Pewnego pięknego dnia starając się nie pozwolić córce na zabawę starym kablem, który wyciągała z uciechą z szafy ukłułem się w palec pinem z zakurzonej maliny.
Przypomniał mi się post, który niedawno czytałem o sterowaniu z rusta [czujnikiem wilgoci i temperatury](https://citizen-stig.github.io/2020/05/17/reading-temperature-sensor-in-rust-using-raspberry-pi-gpio.html). Hmm a gdyby tak...

Na początek skrócona lekcja elektroniki i prosty projekt.
Bardzo polecam przejście przez dowolną serię dobrze opisanych eksperymentów przed zabawą z maliną. Wiedza na temat tego co się dzieje i jak działa prąd zwiększy bezpieczeństwo (nasze i delikatnych układów scalonych) oraz zapewni dużo większą satysfakcję z całego procesu.

## Zakupy i przygotowania...

Dalej drobna lista zakupów:

- Raspberry pi (właściwie dowolny model, ja posiadam 2B)
- karta pamięci zgodna z wymaganiami maliny
- karta rozszerzeń GPIO
- Taśma 40-pin do karty GPIO
- płytka stykowa
- przewody męsko męskie
- dioda led
- rezystor o odpowiedniej wartości w moim przypakdu 4,7kohm (potem zamieniony na 330)

W pierwszej kolejności należy zadbać o to, żeby na karcie pamięci pojawił się zainstalowany aktualny system.
Można podejść do tego zgodnie z [instrukcją producenta](https://www.raspberrypi.org/documentation/installation/installing-images/).
Podstawowe dane logowania to `pi` i hasło `raspberry`. Za pierwszym razem musimy się zalogować przy użyciu wyjścia hdmi i klawiatury fizycznej.
Aby umożliwić wygodną dalszą pracę warto zadbać o możliwość zdalnego połączenia poprzez włączenie [usługi ssh na malinie](https://www.raspberrypi.org/documentation/remote-access/ssh/).

Dodatkowo zakładam, że kod napisany w rust będę uruchamiał na swoim laptopie z linuxem.
Możliwe jest wykonanie tego samego procesu używając dowolnego systemu a nawet na samej malinie.
Wybieram model pracy z kompilacją na swoim laptopie ze względu na czas kompilacji i obecność wszystkich wymaganych narzędzi.

Wymagane narzędzia do pracy z kodem w rust:

- narzędzia do [kompilacji rust](https://rustup.rs/)
- dowolny edytor tekstu, polecam VScode z wtyczką Rust Analyzer
- ssh i scp do połączenia zdalnego i przesłania skompilowanego projektu

## Czas zakasać rękawy

Do maliny podłączamy taśmę gpio jak na zdjęciu:

{{< figure src="/images/rustypi/leds/taśma.jpg" class="img-sm">}}

A następnie taśmę do płytki rozszerzeń i zamontować ją na płytce stykowej.
Pełen układ jest widoczny poniżej:

{{< figure src="/images/rustypi/leds/led_bb.svg" class="img-sm">}}

Oraz na zdjęciu

{{< figure src="/images/rustypi/leds/układ.jpg" class="img-sm">}}

- **Pin GPIO23** jest połączony czerwonym przewodem z szyną dodatnią
- **PIN GND** jest połączony z szyną ujemną
- dioda LED (krótsza nóżka powinna być połączona z ujemną szyną)
- rezystor o wartości 4,7K ohm (taki miałem pod ręką)
- przewód zamykający obwód

Aby przetestować układ możemy uruchomić skrypt (na malinie):


```python
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
W tym momencie dioda przestanie się palić.

## Ale co właściwie się stało

Aby zrozumieć co właściwie się dzieje można przeczytać kod źródłowy, [dokumentację](https://www.raspberrypi.org/documentation/hardware/raspberrypi/bcm2835/BCM2835-ARM-Peripherals.pdf) albo... Hmmm, zaimplementować całość ręcznie w C (po raz pierwszy w życiu).

Wejście wyście ogólnego przeznaczenia - GPIO to cyfrowy interfejs komunikacji między elementami mikroprocesora a urządzeniami peryferyjnymi jak nasza dioda. Interfejs ten jest dostępny dla procesora jako zakres adresów w pamięci.

Istnieją dwa sposoby komunikacji:

- `/dev/mem` (oraz bardziej bezpieczny `/dev/gpiomem`)
- `sysfs` - pseudo system plików dostarczany wraz z jądrem linuksa  

Ostatni sposób jest bardzo prosty i polega na manipulowaniu plikami:

```bash
echo 23 > /sys/class/gpio/export
echo out > /sys/class/gpio/gpio23/direction 
echo 1 > /sys/class/gpio/gpio23/value
echo 0 > /sys/class/gpio/gpio23/value
```

Za zapalanie naszej diody odpowiada w tym przypadku kernel. Jednak wadą jest brak kontroli nad momentem wykonania operacji.
Nie ma to wielkiego znaczenia kiedy chcemy zapalać diodę LED, ale może mieć ogromne znaczenie, jeżeli nasze układy staną się bardziej złożone i zależne od czasu.

Aby sterować w naszym programie pinem GPIO użyjemy pierwszego sposobu przy użyciu pliku `/dev/gpiomem`. W pierwszej kolejnośći należy otworzyć jeden ze wskazanych plików i użyć wywołania systemowego `mmap` które spowoduje, że system odwzoruje ten plik w przestrzeni adresowej pamięci procesu.

Od tego momentu plik z perspektywy naszego programu wygląda jak zwykła tablica bajtów, nie musimy wykorzystywać innych wywołań systemowych do odczytu czy zapisu.

Ufff... Dużo gadania, ale czy ten super prosty skrypt pythonowy też musiał się tak męczyć?
Żeby to sprawdzić bez wczytywania się w dokumentację biblioteki czy kodu możemy wykorzystać system operacyjny.


```bash
pi@raspberrypi:~ $ python led.py 
^CTraceback (most recent call last):
  File "led.py", line 11, in <module>
    sleep(1)
KeyboardInterrupt
closing _devices_shutdown
pi@raspberrypi:~ $ python led.py 
this is weird
^Z
[2]+  Zatrzymano              python led.py
pi@raspberrypi:~ $ lsof -p $(pidof python)
COMMAND  PID USER   FD   TYPE DEVICE SIZE/OFF   NODE NAME
python  4178   pi  cwd    DIR  179,2     4096 263116 /home/pi
python  4178   pi  rtd    DIR  179,2     4096      2 /
python  4178   pi  txt    REG  179,2  2984816 271183 /usr/bin/python2.7
python  4178   pi  mem    REG  179,2    42908  20121 /usr/lib/python2.7/dist-packages/RPi/_GPIO.arm-linux-gnueabihf.so
python  4178   pi  mem    REG  179,2  3031504 274369 /usr/lib/locale/locale-archive
python  4178   pi  mem    REG  179,2  1296004    427 /lib/arm-linux-gnueabihf/libc-2.28.so
python  4178   pi  mem    REG  179,2   464392    452 /lib/arm-linux-gnueabihf/libm-2.28.so
python  4178   pi  mem    REG  179,2   108168    514 /lib/arm-linux-gnueabihf/libz.so.1.2.11
python  4178   pi  mem    REG  179,2     9796    511 /lib/arm-linux-gnueabihf/libutil-2.28.so
python  4178   pi  mem    REG  179,2     9768    435 /lib/arm-linux-gnueabihf/libdl-2.28.so
python  4178   pi  mem    REG  179,2   130416    491 /lib/arm-linux-gnueabihf/libpthread-2.28.so
python  4178   pi  mem    REG  179,2    19876  19899 /usr/lib/python2.7/dist-packages/spidev.arm-linux-gnueabihf.so
python  4178   pi  mem    REG  179,2    17708 272881 /usr/lib/arm-linux-gnueabihf/libarmmem-v7l.so
python  4178   pi  mem    REG  179,2   138604    352 /lib/arm-linux-gnueabihf/ld-2.28.so
python  4178   pi  mem    CHR  247,0            1116 /dev/gpiomem
python  4178   pi    0u   CHR  136,0      0t0      3 /dev/pts/0
python  4178   pi    1u   CHR  136,0      0t0      3 /dev/pts/0
python  4178   pi    2u   CHR  136,0      0t0      3 /dev/pts/0
python  4178   pi    3u   CHR  247,0      0t0   1116 /dev/gpiomem
```

Włączamy nasz skrypt i naciskamy CTRL+Z wstrzymując proces poprzez wysłanie sygnału `SIGSTOP`.
Następnie używając polecenia `lsof -p $(pidof python)` znajdujemy listę plików otwartych przez proces.
Ha! `/dev/gpiomem` znajduje się na liście!

W takim razie spróbujemy ręcznie używając C:

```c
#include <stdio.h>    // for printf
#include <fcntl.h>    // for open
#include <sys/mman.h> // for mmap
#include <unistd.h>

#define GPCLR0 0x28
#define GPSET0 0x1C
#define GPLEV0 0x34

/* 
    Sprawdza stan pina o numerze 23, ustawia stan na wysoki po czym zmienia na niski

    https://www.raspberrypi.org/documentation/hardware/raspberrypi/bcm2835/BCM2835-ARM-Peripherals.pdf
    https://www.cs.uaf.edu/2016/fall/cs301/lecture/11_09_raspberry_pi.html
    https://elinux.org/RPi_GPIO_Code_Samples
 */
int main()
{
    int fd = open("/dev/gpiomem", O_RDWR);
    if (fd < 0)
    {
        printf("Error opening /dev/gpiomem");
        return -1;
    }

    unsigned int *gpio = (unsigned int *)mmap(0, 4096,
                                              PROT_READ + PROT_WRITE, MAP_SHARED,
                                              fd, 0);

    int gpio_num = 23;

    // offset w bajtach pomiędzy kolejnymi elementami w tablicy
    // 32 bity = 8 * 4 bajty
    int u32_offset = 4;

    int FSEL_SHIFT = (gpio_num) / 10;

    // każdy pin ma przypisane 3 bity
    // 000 -> input
    // 001 -> output
    // 010 i wyżej -> alternate functions (zależne od numeru pinu)
    //
    // więcej w rozdziale 6.2
    gpio[FSEL_SHIFT] &= ~(7 << (((gpio_num) % 10) * 3)); // zawsze przed ustawieniem na output musimy ustawić na input
    gpio[FSEL_SHIFT] |= (1 << (((gpio_num) % 10) * 3));  // output

    // GPLEV0 piny 0 - 31, ten kod nie obsłuży pin > 31
    // Odczytuje stan pina gpio_num poprzez odczytanie bitu gpio_num rejestru GPLEV0
    int state = (gpio[GPLEV0 / u32_offset] >> gpio_num) & 1;
    printf("status is %i\n", state);

    while (1==1)
    {
        sleep(1);

        gpio[GPSET0 / u32_offset] |= 1 << gpio_num;
        printf("set to high\n");

        sleep(1);

        gpio[GPCLR0 / u32_offset] |= 1 << gpio_num;
        printf("set to low\n");
    }

    return 0;
}

```


Układ scalony BCM2835 posiada 41 rejestrów, każdy z nich ma 32 bity. Aby mieć do nich dostęp w pierwszej kolejności otwieramy plik `/dev/gpiomem`

```c
int fd = open("/dev/gpiomem", O_RDWR);
```

Oraz używamy `mmap` aby mieć możliwość operowania na jego zawartości:

```c
    unsigned int *gpio = (unsigned int *)mmap(0, 4096,
                                              PROT_READ + PROT_WRITE, MAP_SHARED,
                                              fd, 0);

```

Zgodnie z dokumentacją aby odczytać stan danego pina (w naszym przypadku 23) należy odczytać odpowiedni bit rejestru `GPLEV0` który posiada adres `0x 7E20 0034` (offset `0x34`):

```c
int state = (gpio[GPLEV0 / u32_offset] >> gpio_num) & 1;
```

Jeżeli chcemy zmienić stan danego pina musimy najpierw zmienić jego tryb na wyjściowy.
Zgodnie z dokumentacją każdy z 54 pinów posiada przynajmniej dwie funkcje. 
Przykładowo, żeby ustawić tryb pracy pina 23 (a pozostałych na domyślną wartość 000) należy ustawić wartość rejestru `GPFSEL2` (rejestr dla pinów 20-29) na 001

00000000000000000000**001**000000000

W kodzie przed ustawieniem trybu wyjściowego najpierw ustawiam na tryb wejściowy podążając za poradami z przykładów.
Następnie aby ustawić wartość pina na wysoką należy ustawić odpowiedni bit w rejestrze `GPSET{n}` gdzie `n==0` dla pinów 0-31.

```c
gpio[GPSET0 / u32_offset] |= 1 << gpio_num;
```

Aby zgasić naszą diodę należy ustawić odpowiedni bit innego rejestru: `GPCLR{n}` gdzie `n==0` dla pinów 0-31:

```c
gpio[GPCLR0/u32_offset] |= 1 << gpio_num;
```

Plik możemy skompilować na malinie używając bibliotek dostarczanych razem z systemem operacyjnym:

```bash
pi@raspberrypi:~ $ gcc led.c -o led
pi@raspberrypi:~ $ ./led 
```

## Co jeżeli nie jestem wielkim fanem C?

To już w sumie 3 różne sposoby na wykonywanie tej samej - mało potrzebnej czynności.
Co jeżeli manipulowanie plikami nam nie odpowiada, nie chcemy tracić zalet rozwiązania z C a języki interpretowane zachęcają nas tak samo jak głaskanie jeża pod włos? **Pora na kolejną technologię!**

{{< figure src="/images/rystypi/leds/it-crowd-gif-4.gif" class="img-lg">}}

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
Zakładając, że znamy adres ip maliny (w moim przypadku `192.168.8.103`) oraz, że skonfigurowaliśmy odpowiednie klucze prywatne i publiczne możemy go przesłać używając polecenia:

```bash
# przesyłamy plik
scp target/armv7-unknown-linux-gnueabihf/debug/rusty_led pi@192.168.8.113:~/
# uruchamiamy skompilowany plik wykonywalny
ssh pi@192.168.8.113 './rusty_led'

#   output:
#   Hello, world!
```

Teraz należy dodać zależność do biblioteki która umożliwi nam używanie GPIO.
Zeby to zrobić starczy dopisać do pliku `Cargo.toml` w sekcji `[dependencies]`:


```toml
rppal = "0.11.3"
```

A treść pliku `main.rs` zamienić:

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

Po skompilowaniu, przesłaniu i wykonaniu programu na malinie osiągamy analogiczny efekt do kodu z pythona i C.
Jeżeli zaglądniemy dokładniej w kod źródłowy biblioteki rppal to zauważymy pewne podobieństwa do programu, który napisaliśmy w C: 


```rust
    fn map_devgpiomem() -> Result<*mut u32> {
        // Open /dev/gpiomem with read/write/sync flags. This might fail if
        // /dev/gpiomem doesn't exist (< Raspbian Jessie), or /dev/gpiomem
        // doesn't have the appropriate permissions, or the current user is
        // not a member of the gpio group.
        let gpiomem_file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_SYNC)
            .open(PATH_DEV_GPIOMEM)?;

        // Memory-map /dev/gpiomem at offset 0
        let gpiomem_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                GPIO_MEM_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                gpiomem_file.as_raw_fd(),
                0,
            )
        };

        if gpiomem_ptr == MAP_FAILED {
            return Err(Error::Io(io::Error::last_os_error()));
        }

        Ok(gpiomem_ptr as *mut u32)
    }
```

Hmmm pomijając o wiele ładniejszą kontrolę błędów kod jest właściwie taki sam! Wykorzystuje crate (biblioteka w środowisku rusta) `libc` do wywołania w bloku `unsafe` tego samego wywołania systemowego. Podobnie jest w przypadku pozostałych fragmentów, przykładowo ustawienia wartości pin:

```rust
    const GPSET0: usize = 0x1c / std::mem::size_of::<u32>();

    // some parts omitted

    pub(crate) fn set_high(&self, pin: u8) {
        let offset = GPSET0 + pin as usize / 32;
        let shift = pin % 32;
        self.write(offset, 1 << shift);
    }
```


Jest jeden problem, zachowanie naszego układu elektronicznego po wyjściu z programu jest niedeterministyczne (podobnie jak w przypadku programu w C). Przez wyjście mam na myśli naciśnięcie CTRL+C czyli wysłanie sygnału `SIGINT`.
Obydwa programy (C i rust) nie czyszczą w żaden sposób stanu, dioda zostaje włączona jeżeli w momencie wysłania sygnału była właśnie w takim stanie. 

Czemu tak jest? Jedynie biblioteka w pythonie którą używaliśmy posiada taką funkcjonalność.
Fragment kodu który za nią odpowiada można znaleźć w pliku `devices.py` 

```python
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
```

Bardzo sympatyczna funkcjonalność tylko co jeżeli jej nie chcemy?
Okazuje się, że nie jest to łatwe, o czym można poczytać [tu](https://github.com/gpiozero/gpiozero/issues/707) czy [tu](https://stackoverflow.com/questions/53618198/).


Co jednak jeżeli chcemy dodać podobną funkcjonalność do naszego nowego kodu w rust? 
Zgodnie z dokumentacją crate [rppal](https://docs.rs/rppal/0.11.3/rppal/gpio/index.html#pins) pin powinien zostać przywrócony do stanu oryginalnego w momencie kiedy zgodnie z zasadami własności obiekt zostaje porzucony. Ale co właściwie oznacza porzucenie? 


Rust posiada innowacyjne podejście do zarządzania pamięcią poprzez wprowadzenie modelu własności.
Zarządzanie pamięcią nie jest jak w przypadku C czy C++ zarządzane ręcznie przez programistę, czy jak w javie czy go przez osobny obiekt nazywany garbage collection. O momencie zwalniania pamięci decyduje zestaw reguł, które kompilator częsciowo jest w stanie wywnioskować sam na podstawie ogólnych zasad lub z użyciem parametrów przekazywanych przez programistę w nieoczywistych przypadkach.


```rust
use std::thread;
use std::time::Duration;

struct HasDrop{
    pub name: u32
}

// 
impl Drop for HasDrop {
    fn drop(&mut self) {
        println!("Dropping {}", self.name);
    }
}

fn main() {
    {
        let _x = HasDrop{name: 1};
    } // _x zostaje porzucone w tym miejscu
    let _y = HasDrop{name: 2};
    thread::sleep(Duration::from_millis(5000));
} // _ y w tym miejscu jeżeli wcześniej nie wysłany zostanie SIGINT
```


```shell
$ cargo -q run --example drop
Dropping 1
Dropping 2
$ cargo -q run --example drop
Dropping 1
^C
# po uzyskaniu SIGINT brak wpisu o wykonaniu metody drop na y
```


Model ten daje ogromne korzyści i może być wykorzystywana również do ciekawych zastosowań, jak na przykład oddawanie połączenia do bazy danych do pooli po jego wykorzystaniu bez pisania niepotrzebnego kodu.
W ten sam sposób zorganizowane jest przywracanie stanu oryginalnego dla pinów w rppal:

```rust
            fn drop(&mut self) {
                if !self.reset_on_drop {
                    return;
                }

                if let Some(prev_mode) = self.prev_mode {
                    self.pin.set_mode(prev_mode);
                }

                if self.pud_mode != PullUpDown::Off {
                    self.pin.set_pullupdown(PullUpDown::Off);
                }
            }
```

Jednak jak udowodniliśmy wyżej metoda drop nie jest wołana w sytuacji kiedy przyczyną wyjścia z programu był sygnał `SIGINT`.
Czy jesteśmy w stanie coś z tym zrobić? Zgodnie z sugestią autora biblioteki musimy ręcznie obsłużyć odpowiedni sygnał. W pierwszej kolejności dodajemy nową bibliotekę do Cargo.toml: 


```toml
ctrlc = "3.1.4"
```

A następnie modyfikujemy nasz program:

```rust
use std::error::Error;
use std::thread;
use std::time::Duration;

use rppal::gpio::Gpio;
use rppal::system::DeviceInfo;

use std::sync::{Arc, Mutex};

//https://docs.golemparts.com/rppal/0.11.2/rppal/gpio/struct.OutputPin.html#note

fn main() -> Result<(), Box<dyn Error>> {
    println!("Działam na {}.", DeviceInfo::new()?.model());

    let mut pin = Gpio::new()?.get(23)?.into_output();
    let closed = Arc::new(Mutex::new(false));

    let closed_handler = closed.clone();
    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        println!("set to closed");
        *closed_handler.lock().unwrap() = true;
    })?;

    while !*closed.lock().unwrap() {
        pin.set_high();
        thread::sleep(Duration::from_millis(500));
        pin.set_low();
        thread::sleep(Duration::from_millis(500));
    }

    println!("setting to low");
    pin.set_low();
    Ok(())
}
```

Dodaliśmy zmienną `closed` zabezpieczoną `Arc` - mądrym wskaźnikiem pozwalającym na dzielenie fragmentu pamięci pomiędzy wątkami i `Mutex` który zabezpiecza do niego dostęp. Czemu nie dodamy po prostu ustawienia stanu pina na niski w handlerze sygnału? 

```rust
    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        pin.set_low();
    })?;
```

Odpowiedź brzmi ponieważ kompilator rusta nam na to nie pozwoli. Otrzymamy błąd:

```
error[E0382]: borrow of moved value: `pin`
  --> src/main.rs:26:9
   |
15 |     let mut pin = Gpio::new()?.get(23)?.into_output();
   |         ------- move occurs because `pin` has type `rppal::gpio::pin::OutputPin`, which does not implement the `Copy` trait
...
19 |     ctrlc::set_handler(move || {
   |                        ------- value moved into closure here
...
22 |         pin.set_low();
   |         --- variable moved due to use in closure
...
26 |         pin.set_high();
   |         ^^^ value borrowed here after move

error: aborting due to 2 previous errors
```

Czyli zadziałają zasady własności które ochronią nas przed błędem, który nie zostałby wychwycony w analogicznym kodzie w C. Na czym polega błąd? Kod wykonywany po otrzymaniu sygnału `SIGINT` jest wykonywany w innym wątku niż pętla `while`
co oznacza, że potencjalnie wystąpiłby wyścig i zachowanie programu mogło by być różne. 



## Wnioski

Nauczyliśmy się w jaki sposób można sterować pinami maliny oraz wielu sposobów zapalania diody LED.
Z moich doświadczeń z budowania tak prostego ukłądu wynika, że potrzebna jest duża uwaga, żeby nie popełnić błędu.
Łatwiej jest znaleźć i rozwiązać błąd jeżeli taki się pojawi w prostszym układzie.
Wraz ze zwiększaniem się rozmiarów kodu sterującego coraz bardziej zaawansowanymi układami prawdopodobieństwo błędów się powiększa. 


Dlatego w mojej ocenie używanie narzędzi jak rust ma ogromną przewagę nad C, w którym brakuje silnego typowania oraz obsługi błędów. Po drugiej stronie jest python, który bardzo pasuje do maliny, natomiast jego użyteczność zmniejsza się na mniejszych układach, gdzie dostępna pamięć jest zdecydowanie większym ograniczneniem.


