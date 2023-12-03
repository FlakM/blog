---
title: "The impossible case of pitching rust in a web dev shop"
date: 2022-09-18T12:12:24+02:00
draft: false
authors: ["Maciej Flak"]
images: [
    "/images/scales.jpg",
    ]

featured_image: "/images/scales.jpg"
tags: ["rust", "english"] 

Summary: '
Short research about making decision to use rust in medium web development shop.
' 
toc: false

---

# Pitching rust to a mid-sized web dev shop

I crave working with rust. [So do many other people](https://survey.stackoverflow.co/2022/#most-loved-dreaded-and-wanted-language-love-dread).
I wrote down my thoughts because I know I'm biased.

I want like to tell a story of a fictional **middle-sized web development company**.
They use high-level, battle-tested technologies JVM, c#, go, ruby, or python.

They follow all the industry standards like code review, CI/CD, feature
flags, and automated security scans...
But an essential piece of the puzzle:

> **successful product, active users, and actual revenue**

With this context in mind, let's look at rust's selling points.

- safety
- blazing fast
- fearless concurrency
- productivity

*But wait*, those are marginally tempting for them.

No need to be blazingly fast - fast is good enough. 
The GC languages (or ref counted) are already mostly safe. 
Ecosystem libraries allow you to be fearless with concurrency.
Productive? They have become an established company - hard to argue with
that. 


**Is it possible that rust does not fit this scenario?**

## The problems with adoption

Let's think about possible problems

1. developers need to **spend time learning** new things
2. create new CI/CD pipelines with caches and security scanning
3. **alternative cost** - devs will not create new features when learning
4. the organization will have to **learn how to teach and hire** new talent

Point number one gets even more complicated when discussing about classic web companies. 
They are not used to the terminology commonly used with rust and c family.
Pointers, allocations, and dynamic/static linking are enough to make a head spin.
Those concepts are conveniently abstracted away.

Crating CI/CD infrastructure and pipelines is not a small feat,
especially when the old ones have been polished so much over the years.

Cargo is fantastic, but many years went into maven, gradle, sbt, pip, poetry, conda...
They have their problems - but also have significant benefits - we understand them

Lastly, there is an issue, and it's a big one. The company is sacrificing the possible features that could have been delivered at the same time.
Depending on the competition in the market, an initial slowdown might cause significant churn.

## The potential benefits

It's not all bad, is it? 

Serious players like Microsoft, AWS, and Meta are not exactly jumping on every hype train (they create their own).

They all have different business use cases that might fit high performance better than classic web companies.
I can see the following main reasons for adopting rust in described context:

1. Optimizing for **the long ride**
2. Obtaining a **highly versatile skill**
3. **Encouraging learning** and not being afraid of lower abstractions
4. **Saving $$$**
5. Encouraging good security practices by using sane defaults and exposing the problems to end users

### The long ride argument

Did you know that the programming language C was created by Dennis Ritchie 50 years ago?
To prove my point, Linus Torvalds just mentioned that [rust will be included in Linux 6.1](https://www.zdnet.com/article/linus-torvalds-rust-will-go-into-linux-6-1/), 
and for instance, Google uses rust to [develop Android](https://security.googleblog.com/2021/04/rust-in-android-platform.html).
In my opinion, Rust has a similar vibe of language for the next decades due to the following features:

- **Statically typed** - catches some problem in logic as soon as possible
- **Expression-based** - inspired by OCaml (first compiler version was written in [it](https://github.com/rust-lang/rust/tree/ef75860a0a72f79f97216f8aaa5b388d98da6480/src/boot))
- **Immutability and mutability at the type level** - allows specifying contracts to ease the future modification
- **It doesn't hide the complexity**. For instance, it has Result/Option types through the standard library
- **Rich documentation tooling** autogenerated from code that encourages documentation
- Ownership model not only helps with avoiding problems also solved by GC but also with common **concurrency issues**
- **Tooling is superb** - it has a powerful package manager, linter, formater, and compiler with outstanding error messages with helpful suggestions
- It does not have a company behind it but [the foundation](https://foundation.rust-lang.org/)
- **Very popular open source project** with over [4 thousand contributors!](https://github.com/rust-lang/rust/graphs/contributors)
- **A rich ecosystem** of libraries (even though some of them have few contributors)
- It has a very cool jingle in my favorite [Linux podcast](https://www.jupiterbroadcasting.com/) in the picks section

**We read more code than we write**. 

Rust code is easy to understand and maintain over time.
You change and refactor often and without being afraid. The compiler has your back!

### Versatility

Rust is a beast when it comes to different use cases. In the context of web development, we can use it on:

- Web servers using a vast amount of networking protocols like [http](https://github.com/tokio-rs/axum), [grpc](https://github.com/hyperium/tonic) or access popular storage like [s3](https://github.com/rusoto/rusoto)
- Mobile phones for instance on [ios](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-06-rust-on-ios.html) or [Android](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-21-rust-on-android.html)
- Terminal as very powerful cli tools
- GUI applications 
- On browser using [WebAssembly](https://rustwasm.github.io/docs/book/)
- Run inside other applications to power some performance-critical sections using bindings [(python)](https://github.com/PyO3/pyo3)

**Developers can contribute to new areas** that would be previously unavailable.

### Encouraging learning

The unique mix of traits makes rust **encouraging to experiment** with. 

It's thanks to endless efforts of the community to polish the tooling like compiler messages or linters.

For me, it is the documentation, though. It is a genuine pleasure to read and write each time. 

To showcase let us see my favorite examples:

- [documentation of mutex in the standard library](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
- [root documentation of popular cli argument parsing library](https://docs.rs/clap/latest/clap/)
- [query macro documentation for popular sql library](https://docs.rs/sqlx/latest/sqlx/macro.query.html)

### Saving on the cloud bill

This one is tricky to measure.

The wrong data structure or algorithm written in the best language would still gobble on those precious watts.

> Obligatory disclaimer! **Rust is not in any way a panacea to all problems!** 
I've managed to create a surprising number of bugs. It is even more impressive with a relatively low number of lines written. 

Depending on the scenario, there might be different factors of high invoice:
significant network traffic, big needs for computing power (CPU-bound tasks), high memory needs, storage costs, and many more.

Even though it's probably hard to estimate the efficiency of a language itself in a complex web company,
some [sources](https://greenlab.di.uminho.pt/wp-content/uploads/2017/10/sleFinal.pdf) try to create synthetic metrics.

Benchmarks show that the rust/c program can be up to 2 times more power efficient than java or even 75 times than python.
Large, well-established companies start to share their results after adopting rust:

- [Discord](https://discord.com/blog/why-discord-is-switching-from-go-to-rust) reduced the application's memory footprint enjoyed another benefit of using rust memory model: they eliminated latency spikes that were caused by GC in the original go implementation.
- [AWS](https://aws.amazon.com/blogs/opensource/sustainability-with-rust/) estimated that rust is an essential tool to reduce total worldwide data center energy consumption, which amount to 1% of total energy usage.
- [Cloudflare](https://www.phoronix.com/news/CloudFlare-Pingora-No-Nginx) shared outstanding savings after migrating their nginx proxy to rust. Cloudflare stated that they could reduce CPU load by 70% and use 76% less memory.

There is a common trait between all of those examples:

- **core part of the business value**
- **high load** on the application
- **latencies** directly correlated to the end-user experience

Additionally, I can think of things in the work that would very much benefit this even further:

- iouring adoption in rust ecosystem to support asynchronous computations (networking, fast storage access, and more)
- the recent stabilization of GAT will enable further improvements in asynchronous features in the language. 
  This will empower the async ecosystem to experiment with new cool things.


### Encouraging security and sanity

The main selling point of rust - memory safety does not directly apply here: both java and go are garbage collected, and python uses reference counting - they kind of are ok in this department.

But memory safety is not the only way we shoot ourselves in the foot on a daily basis. 
Rust's ownership model lends itself nicely to two essential topics in modern development: concurrency and parallelism. 

All big players are flirting with the smaller cores on the CPU.
[ARM](https://en.wikipedia.org/wiki/ARM_big.LITTLE), [Intel](https://arstechnica.com/gadgets/2021/11/intels-alder-lake-big-little-cpu-design-tested-its-a-barn-burner/), [Apple](https://developer.apple.com/news/?id=vk3m204o) all have them.
Even cloud vendors offer ARM on the server side like [AWS's graviton](https://aws.amazon.com/ec2/graviton/) or [Microsoft's one](https://azure.microsoft.com/en-us/blog/now-in-preview-azure-virtual-machines-with-ampere-altra-armbased-processors/).
Also, the RISC-V interest seems to hit the [sky](https://www.sifive.com/press/nasa-selects-sifive-and-makes-risc-v-the-go-to-ecosystem) (sorry for the dad joke...).
It's way more cost-effective if you can benefit from those improvements.

Additionally, rust is statically typed and monadish types like Result or Option are sprinkled everywhere in the standard library and the ecosystem with some excellent safety measures.

TLDR for example rustc will get [quite mad](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-must_use-attribute) at you if you decide not to use the outcome of some calculation that might either finish successfully or with an error. 
Or it will give you [very nice errors](https://rustc-dev-guide.rust-lang.org/backend/implicit-caller-location.html) when you mess up some invariant,
for example, unwrap Result with error inside.

But rust is not a solution to all the problems. You can opt out of some safety guarantees, and you will have to many times.
On a happy note though clippy - rust linter will also [scream](https://rust-lang.github.io/rust-clippy/master/#missing_safety_doc) at you if you decide not to document the safety rules that consumers of unsafe functions/methods must uphold.

## Finishing thoughts

Rust is here to stay. It is a perfect tool to have in your pocket as an individual and an organization. 

There are contexts where adopting rust is eased by possessing previous experience with similar technologies.
Sadly, in the context of the average web development shop, **the cost of adopting rust for a new project is extremely high**, even higher when rewriting a big chunk of codebases.

On the other hand, in some cases adopting **rust might provide game-changing value** to developers and the company itself.

I guess if you read to this point - you wasted your time. There is no one correct answer.
