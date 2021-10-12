# Ideas

## Seria postów tłumaczących pewne pojęcia jak dla juniora

1. Co to jest alokacja
2. Co to jest wskaźnik
3. Co to jest FFI
4. dynamic lib loading - przykład użycia biblioteki z pythona/c/javy
5. 

## Nowy post opisujący jak anlizować ASM i co to znaczy alokacja

Może uda się opisać tooling pozwalający do analizy tego w taki sposób:

https://godbolt.org/

Tworzenie:

```
cargo new --bin rust-asm-reading
cd rust-asm-reading/
cargo rustc -- --emit asm
cat target/debug/deps/rust_asm_reading-22f5352283a5d65d.s
```

## Linux - epoll iouring

jak wykorzystać na przykład do czytania plików i co daje np względem czytania synchronicznego.
Na przykład wywołanie synchronicznego read z fd jak nic w nim nie ma zablokuje na zawsze

