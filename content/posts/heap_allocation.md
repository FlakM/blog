---
title: "Heap allocation"
date: 2022-03-07T14:12:20+01:00
draft: true
authors: ["Maciej Flak"]
images: [
    "/images/rustypi/lcd/20200718_204650.jpg",
    ]

tags: ["rust", "english"] 
Summary: '
How to profile heap memory usage in rust application.
' 
---

## Why measure memory usage

Memory is costly, cloud providers differentiate the services based on
CPU/disk/network and memory.
Measuring how much memory application takes is a very useful skill.


## Example problem

The premise of this blog post is profiling piece of rust application.
Application is reading potentially large csv into memory and than storing headers
and fields for later access by index. Let's start an example project:

```
cargo new --bin heaptrace_csv
```

The structures for above problems:

```rust
// src/main.rs
struct DataRecord {
    fields: Vec<String>,
}

struct DataSheet {
    headers: Vec<String>,
    rows: Vec<DataRecord>,
}
```

And the algorithm itself:


```rust
// src/main.rs
fn split_str(s: String) -> Vec<String> {
    s.split(',').map(str::to_string).collect()
}

fn main() {
    let file = std::fs::File::open("./example.csv").unwrap();
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let _sheet = DataSheet {
        headers: split_str(lines.next().unwrap().unwrap()),
        rows: lines
            .map(|l| DataRecord {
                fields: split_str(l.unwrap()),
            })
            .collect(),
    };
}
```

Program creates `BufReader` buffering lines form `example.csv` which is:

```bash
$ du -h example.csv
976K    example.csv
$ wc -l example.csv
11742 example.csv
$
```

## Measuring

To measure memory usage of a process on linux we might use `pmap` but
to do so we have to add something that will make our program wait for
the measuring. To do so lets add simple sleep at the end of main method:

```rust
// end of main method
    std::thread::sleep(std::time::Duration::from_secs(10))
```

So to ease the process of understanding memory address we might want to
disable [ASLR](https://en.wikipedia.org/wiki/Address_space_layout_randomization)
by executing:

```bash
# read https://askubuntu.com/questions/318315/how-can-i-temporarily-disable-aslr-address-space-layout-randomization
# for more information
echo 0 | sudo tee /proc/sys/kernel/randomize_va_space
# to enable it back on after having fun: 
echo 2 | sudo tee /proc/sys/kernel/randomize_va_space
```


And then we are able to measure the output using `pmap`:

```
$ cargo run -q --release &
$ pmap -x $(pgrep heaptrace_csv)
542776:   /home/flakm/programming/flakm/rustberry/target/release/heaptrace_csv
Address           Kbytes     RSS   Dirty Mode  Mapping
0000555555554000      24      24       0 r---- heaptrace_csv
000055555555a000     220     220       0 r-x-- heaptrace_csv
0000555555591000      52      52       0 r---- heaptrace_csv
000055555559f000      12      12      12 r---- heaptrace_csv
00005555555a2000       4       4       4 rw--- heaptrace_csv
00005555555a3000    5156    5144    5144 rw---   [ anon ]
00007ffff7ce0000     388     276     276 rw---   [ anon ]
00007ffff7d72000       8       8       8 rw---   [ anon ]
00007ffff7d74000     152     148       0 r---- libc-2.33.so
00007ffff7d9a000    1452     904       0 r-x-- libc-2.33.so
00007ffff7f05000     304     164       0 r---- libc-2.33.so
00007ffff7f51000      12      12      12 r---- libc-2.33.so
00007ffff7f54000      12      12      12 rw--- libc-2.33.so
00007ffff7f57000      36      12      12 rw---   [ anon ]
00007ffff7f60000       8       8       0 r---- libdl-2.33.so
00007ffff7f62000       8       8       0 r-x-- libdl-2.33.so
00007ffff7f64000       4       0       0 r---- libdl-2.33.so
00007ffff7f65000       4       4       4 r---- libdl-2.33.so
00007ffff7f66000       4       4       4 rw--- libdl-2.33.so
00007ffff7f67000      28      28       0 r---- libpthread-2.33.so
00007ffff7f6e000      64      64       0 r-x-- libpthread-2.33.so
00007ffff7f7e000      20       0       0 r---- libpthread-2.33.so
00007ffff7f83000       4       4       4 r---- libpthread-2.33.so
00007ffff7f84000       4       4       4 rw--- libpthread-2.33.so
00007ffff7f85000      16       4       4 rw---   [ anon ]
00007ffff7f89000      12      12       0 r---- libgcc_s.so.1
00007ffff7f8c000      72      64       0 r-x-- libgcc_s.so.1
00007ffff7f9e000      12      12       0 r---- libgcc_s.so.1
00007ffff7fa1000       4       0       0 ----- libgcc_s.so.1
00007ffff7fa2000       4       4       4 r---- libgcc_s.so.1
00007ffff7fa3000       4       4       4 rw--- libgcc_s.so.1
00007ffff7fbe000       4       0       0 -----   [ anon ]
00007ffff7fbf000      16       8       8 rw---   [ anon ]
00007ffff7fc3000      16       0       0 r----   [ anon ]
00007ffff7fc7000       8       4       0 r-x--   [ anon ]
00007ffff7fc9000       4       4       0 r---- ld-2.33.so
00007ffff7fca000     156     156       0 r-x-- ld-2.33.so
00007ffff7ff1000      40      40       0 r---- ld-2.33.so
00007ffff7ffb000       8       8       8 r---- ld-2.33.so
00007ffff7ffd000       8       8       8 rw--- ld-2.33.so
00007ffffffdc000     140      20      20 rw---   [ stack ]
ffffffffff600000       4       0       0 --x--   [ anon ]
---------------- ------- ------- ------- 
total kB            8508    7464    5552


$ kill $(pgrep heaptrace_csv)
$
```

The process is using 8508 kB of memory.

So we can filter out some shared libraries (files with extension .so) and
focus on `anon` sections. From
[man](https://docs.oracle.com/cd/E19683-01/816-0210/6m6nb7mhj/index.html) page:

> Memory not relating to any named object or file within the file system
> is reported as [ anon ].
> If the common name for the mapping is unknown,
> pmap displays [ anon ] as the mapping name.


Lets focus on anon pages then:

```
pmap -x $(pgrep heaptrace_csv) | grep anon | awk '{sum+=$2;}END{print sum;}'
5652
```

It sums to 5652kB 5x times the size of the file itself.

Let's add some little pointers printed out to help us identify the
regions:

```rust
let zero = 0;
let vec_start = sheet.rows.as_ptr();
let vec_stop = unsafe { vec_start.offset(sheet.rows.len() as isize) };
let main_ptr = main as *const ();
let stack_ptr = &zero;

println!("vec_start:  {vec_start:p}");
println!("vec_stop:  {vec_stop:p}");
println!("code:  {main_ptr:p}");
println!("stack: {stack_ptr:p}");
std::thread::sleep(std::time::Duration::from_secs(60));
println!("done sleeping...")


// this will output:
// cargo run -q --release
// vec_start:  0x7ffff7ce0010
// vec_stop:  0x7ffff7d24cc8
// code:  0x55555555cef0
// stack: 0x7fffffffbcac
// done sleeping...
```
Let's check system calls:

```
$ strace -o strace.log -f ../target/release/heaptrace_csv
$ grep -E "brk|.*map\(NULL.*ANONYMOUS|m.+map" strace.log 
15442 brk(NULL)                         = 0x5555555a3000
15442 mmap(NULL, 8192, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0) = 0x7ffff7fa4000
15442 mmap(NULL, 8192, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0) = 0x7ffff7d72000
15442 munmap(0x7ffff7fa6000, 115894)    = 0
15442 mmap(NULL, 12288, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS|MAP_STACK, -1, 0) = 0x7ffff7fc0000
15442 brk(NULL)                         = 0x5555555a3000
15442 brk(0x5555555c4000)               = 0x5555555c4000
15442 brk(0x5555555e5000)               = 0x5555555e5000
15442 brk(0x555555608000)               = 0x555555608000
15442 brk(0x555555629000)               = 0x555555629000
15442 brk(0x55555564a000)               = 0x55555564a000
15442 brk(0x55555566b000)               = 0x55555566b000
15442 brk(0x55555568c000)               = 0x55555568c000
15442 brk(0x5555556ad000)               = 0x5555556ad000
15442 brk(0x5555556ce000)               = 0x5555556ce000
15442 brk(0x5555556ef000)               = 0x5555556ef000
15442 brk(0x555555710000)               = 0x555555710000
15442 brk(0x555555731000)               = 0x555555731000
15442 brk(0x555555752000)               = 0x555555752000
15442 brk(0x555555773000)               = 0x555555773000
15442 brk(0x555555794000)               = 0x555555794000
15442 mmap(NULL, 200704, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0) = 0x7ffff7d41000
15442 brk(0x5555557b5000)               = 0x5555557b5000
15442 brk(0x5555557d6000)               = 0x5555557d6000
15442 brk(0x5555557f7000)               = 0x5555557f7000
15442 brk(0x555555818000)               = 0x555555818000
15442 brk(0x555555839000)               = 0x555555839000
15442 brk(0x55555585a000)               = 0x55555585a000
15442 brk(0x55555587b000)               = 0x55555587b000
15442 brk(0x55555589c000)               = 0x55555589c000
15442 brk(0x5555558bd000)               = 0x5555558bd000
15442 brk(0x5555558de000)               = 0x5555558de000
15442 brk(0x5555558ff000)               = 0x5555558ff000
15442 brk(0x555555920000)               = 0x555555920000
15442 brk(0x555555941000)               = 0x555555941000
15442 mremap(0x7ffff7d41000, 200704, 397312, MREMAP_MAYMOVE) = 0x7ffff7ce0000
15442 brk(0x555555962000)               = 0x555555962000
15442 brk(0x555555983000)               = 0x555555983000
15442 brk(0x5555559a4000)               = 0x5555559a4000
15442 brk(0x5555559c5000)               = 0x5555559c5000
15442 brk(0x5555559e6000)               = 0x5555559e6000
15442 brk(0x555555a07000)               = 0x555555a07000
15442 brk(0x555555a28000)               = 0x555555a28000
15442 brk(0x555555a49000)               = 0x555555a49000
15442 brk(0x555555a6a000)               = 0x555555a6a000
15442 brk(0x555555a8b000)               = 0x555555a8b000
15442 brk(0x555555aac000)               = 0x555555aac000
15442 munmap(0x7ffff7ce0000, 397312)    = 0
15442 munmap(0x7ffff7fc0000, 12288)     = 0
```

We can see [vec growth](https://nnethercote.github.io/perf-book/heap-allocations.html#vec-growth)
in action.

The line:

```
15442 mmap(NULL, 200704, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0) = 0x7ffff7d41000
```

Is `mmap` call for the vector of rows growing over threshold where
previously it was stored in the same mapping created by brk call:

```
15442 brk(NULL)                         = 0x5555555a3000
```

Then after the code finished iterating over all lines there was a
following call:

```
15442 mremap(0x7ffff7d41000, 200704, 397312, MREMAP_MAYMOVE) = 0x7ffff7ce0000
```

which coresponds to: (397312 bytes is 388 Kb)

```
00005555555a3000    5156    5144    5144 rw---   [ anon ]
00007ffff7ce0000     388     276     276 rw---   [ anon ]
```

To understand this value we have to analyze the output of our program
again. For `DataSheet` allocations for inner `Vecs` are in different
mapping than the contents of the `Vecs`:

```
vec_start:  0x7ffff7ce0010
vec_stop:  0x7ffff7d24cc8
vec_string:  0x5555555a5e80
```

So to calculate the heap size of `DataSheet` excluding inner strings
itself. We have to use
following equation where `x` is number of rows :

```
size = 24 + x * 24 
```

It gives us exactly 276 Kb for exact length of the file. But since `Vec`
is filled in a runtime it cannot know the exact length without a hint
hence growing algorithm reserves larger and larger memory blocks using heuristic.
The component of a `Vec` containing the information how many elements
could this `Vec` hold without new allocation is called `capacity`.
The capacity that the output of the program gave us was 16384.
So we get (for a system with page size equal to 4096):

```
size = 24 + 16384 * 24 = 393400
size_page_aligned = 397312
size_page_aligned_kB = 388
```

If we factor in the expected size of the `Strings` which is roughly
estimated by the size of the file itself (976K) plus each `String` has
it's own memory cost (24 bytes) which gives us:

```
lines = 11742
columns = 7
size = 7 * 11742 * 24 = 1972656 = 1926kB
total_size = 1926Kb + 976kB = 2902kB
```

## The more elegant solution

The problem with above solution is that we are tracking
syscalls and memory mappings not the actual memory usage.
In happy path for smaller allocations using `malloc` should not result in
syscall but reuse previously booked estate backed by some mapping.
Rust by default uses
[std::alloc::System](https://doc.rust-lang.org/std/alloc/struct.System.html) 
allocator which uses `malloc` internally. One could use [LD_PRELOAD](https://stackoverflow.com/questions/426230/what-is-the-ld-preload-trick)
to trace back the calls to `malloc` and `free`.


Googling for it I've found this [blog
post](https://milianw.de/blog/heaptrack-a-heap-memory-profiler-for-linux.html)
introducing awesome library:
[heaptrack](https://github.com/KDE/heaptrack)
- a heap memory profiler for Linux. Under the hoods heaptrack uses exactly `LD_PRELOAD` trick with smart usage of
`libbacktrace` and `unw_backtrace` to gain context. 
Since it instruments `malloc` and `free` calls it can be used with rust
binaries without any code changes. Given the debug symbols
are enabled you can pinpoint allocations to specific place in code.



```
$ heaptrack  ../target/release/heaptrace_csv
heaptrack output will be written to "/home/flakm/programming/flakm/rustberry/heaptrace_csv/heaptrack.heaptrace_csv.471591.gz"
starting application, this might take some time...
heaptrack stats:
        allocations:            129307
        leaked allocations:     0
        temporary allocations:  120
Heaptrack finished! Now run the following to investigate the data:

  heaptrack --analyze "/home/flakm/programming/flakm/rustberry/heaptrace_csv/heaptrack.heaptrace_csv.471591.gz"
```




https://stackoverflow.com/questions/131303/how-can-i-measure-the-actual-memory-usage-of-an-application-or-process/131346#131346
https://nnethercote.github.io/perf-book/profiling.html
https://rust-analyzer.github.io/blog/2020/12/04/measuring-memory-usage-in-rust.html
https://dev.to/dandyvica/different-ways-of-reading-files-in-rust-2n30
https://fasterthanli.me/articles/small-strings-in-rust

