---
title: "Difficult case of pitching rust - web shop"
date: 2022-09-18T14:12:20+01:00
draft: true
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

# How web heavy organizations can benefit from Rust

My relationship with rust could be described as rather unhealthy. 
Working with it feels like having a trusty old friend by your side punching you in the face any time you decide to do something silly.
Additionally, it made me resent other languages for not doing the same. I blame all of [you!](https://github.com/rust-lang/rust/graphs/contributors).
My strong opinions about the language probably make my judgment rather biased. That's why I decided to write my research before pitching it to anyone.


I'd like to tell a story of a middle-sized web development company (I'm thinking 20-200 devs actively maintaining the code).
To draw the picture here: the fictional company has been mainly using high-level battle-tested technologies either JVM, c#, go, ruby, or python.
To enable rapid growth it accepted some well-known practices and technologies. At the same time, it managed to create a successful product.
Since the architecture decisions are made by a group of very smart people with rather large experience and domain knowledge they will not accept any marketing bs.

With this context in mind let's look at rust's selling points.

- safety
- blazing fast
- fearless concurrency
- productivity

But wait. Those things are marginally tempting for the company.
They don't need to be blazingly fast - fast is good enough. 
The GC languages (or ref counted) are already mostly safe. 
Some libraries allow you to be fearless with concurrency.
Productive? Well, they have been productive - they got to become an established company. 


I know you cannot tell but I'm scared. 
Is it possible that rust does not fit this scenario?

## The problems with adoption

It is challenging to establish valid tactics to measure the impact of using new technology. But the following problems seem obvious:

1. developers need to spend time learning new technologies and styles of programming
2. new CI/CD pipelines with caches and security scanning have to be created
3. alternative cost - developers will not create new features when learning
4. organization will have to learn how to teach and hire new talent

Point number one gets even harder when we are talking about classic web companies - they are not exactly used to terminology that is common with rust and c family.
Pointers, allocations, and dynamic/static linking are enough to make a head spin.
All of the above are present in mentioned languages, but they are abstracted away for our convenience. 

Investing time to create CI/CD pipelines is not a small feat in itself.
Especially when the old ones were polished so much over the years.
There are things that they already do for free, and they are... well established. 

Even though cargo is awesome many years went into maven, gradle, sbt, pip, poetry, conda...
Although some of them have their problems - they also have large benefits - they are well understood.

Lastly, there is an issue and it's a big one. Sacrificing the possible features that could have been delivered at the same time.
Depending on the competition in the market, an initial slowdown might cause major churn.

## The potential benefits

It's not all bad though, is it? There must be something behind the massive hype that I keep on reading and hearing.
Large companies like Amazon and Microsoft that have a lot of experience and investment
in other languages are not willing to thoughtlessly jump to new technologies.
Granted they all have quite different business use cases that might fit high performance better than classic web companies.
I can see the following main reasons for adopting rust in described context:

1. Optimizing for the long ride
2. Obtaining a highly versatile skill - applicable in many contexts
3. Encouraging learning and not being afraid of lower abstractions
4. Saving on the cloud bill 
5. Encouraging good security practices by using sane defaults and exposing the problems to end users

### The long ride argument

Programming language C was created by Dennis Ritchie 50 years ago. 
Even though it's easy to give in to the hype of fast-moving and breaking stuff the reality is that some software will run for a long time - maybe 50 years or more.
Compiled rust program doesn't differ from compiled C program that much (well...). It uses the same underlying technologies, just allows you to avoid common foot guns:

- It is statically typed - this catches some problem in logic as soon as possible
- Its expression based It is inspired by OCaml (it has even been written in [it](https://github.com/rust-lang/rust/tree/ef75860a0a72f79f97216f8aaa5b388d98da6480/src/boot)
- It has a notion of immutability and mutability at the type level - allows to specify contracts to disallow invalid behaviors due to future modifications
- It has Result/Option types through the standard library - it does not hide the complexity
- It has a very rich documentation tooling autogenerated from code that encourages easy documentation. 
  Check out [this](https://doc.rust-lang.org/std/primitive.slice.html#method.swap) it even contains runnable examples in a web browser!
- Ownership model not only helps with avoiding problems introduced by GC but also helps with concurrency
- Tooling is superb - it has a powerful package manager, linter, formater, and compiler with outstanding error messages with helpful suggestions
- It does not have a company behind it but [foundation](https://foundation.rust-lang.org/)
- It is a very popular open source project with over [4 thousand contributors!](https://github.com/rust-lang/rust/graphs/contributors)
- It has a very rich ecosystem of libraries (even though some of them have few contributors)
- It has a very cool jingle in my favorite [Linux podcast](https://www.jupiterbroadcasting.com/) in the picks section

All of those features make a rust source code easy to maintain over time.
It's far easier to introduce change ad refactor if you can trust the compiler to catch all your mistakes.

### Versitality

Rust is a beast when it comes to different use cases. In the context of web development, it can be used to:

- Run as powerful web servers using a vast amount of networking protocols like [http](https://github.com/tokio-rs/axum), [grpc](https://github.com/hyperium/tonic) or access popular storage like [s3](https://github.com/rusoto/rusoto)
- Run on mobile phones for instance on [ios](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-06-rust-on-ios.html) or [Android](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-21-rust-on-android.html)
- Run on the terminal as very powerful cli tools
- Power GUI applications 
- Run on browser using [WebAssembly](https://rustwasm.github.io/docs/book/)
- Run inside python applications to power some performance-critical sections using [bindings](https://github.com/PyO3/pyo3)

Once perfected this could enable a developer to contribute in areas that would previously require a large undertaking of learning new technologies.

### Encouraging learning

The unique mix of traits makes rust encouraging to experiment with. 
It's thanks to endless efforts of the community to polish the tooling like compiler messages or linters.
Personally, the biggest confidence boost might be attributed to the work that goes into documentation tooling and the documentation itself.
In the world of very fast-moving world quality of documentation is very often sacrificed in the name of delivering features.

This is not the case with rust. Just to showcase let us see my favorite examples:

- [documentation of mutex in standard library](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
- [root documentation of popular cli argument parsing library](https://docs.rs/clap/latest/clap/)
- [query macro documentation for popular sql library](https://docs.rs/sqlx/latest/sqlx/macro.query.html)

### Saving on the cloud bill

This one is exceptionally tricky due to the complicated nature of the measurements. 
The wrong data structure or algorithm written in the best language would still gobble on those precious watts.

Obligatory disclaimer - **Rust is not in any way a panacea to all problems!** 
For example, I've managed to create a surprising number of bugs which is impressive taking into consideration that for me rust has been only a side hobby and I've only managed to use it a couple of times professionally  
But let's return to the main point...

Depending on the scenario there might be different factors of high invoice:
significant network traffic, large needs for computing power (CPU-bound tasks), high memory needs, storage costs,, and many more.

Even though it's probably hard to estimate the efficiency of a language itself in a complex web company 
there are [sources](https://greenlab.di.uminho.pt/wp-content/uploads/2017/10/sleFinal.pdf) that try to create synthetic metrics.
Benchmarks show that the rust/c program can be up to 2 times more power efficient than java or even 75 times than python.
Large, well-established companies start to share their results after adopting rust:

- Discord was [able](https://discord.com/blog/why-discord-is-switching-from-go-to-rust) not only to reduce the memory footprint of the application but also to enjoy the benefits of using rust memory model: they eliminated latency spikes that were caused by GC in original go implementation.
- AWS estimated [that](https://aws.amazon.com/blogs/opensource/sustainability-with-rust/) rust is an essential tool to reduce total worldwide data center energy consumption which amounts to 1% of total energy usage.
- [Recently](https://www.phoronix.com/news/CloudFlare-Pingora-No-Nginx) Cloudflare shared outstanding savings after migrating their nginx proxy to rust. Cloudflare stated that they way able to reduce CPU load by 70% and use 76% less memory.

There is a common trait between all of those examples: discussed areas are in the very **core of the business**, there is a **high load** on the application, and the **latencies** are directly correlated to the end-user experience.

Additionally, I can think of things in the work that would very much benefit this even further:

- iouring adoption in rust ecosystem to support asynchronous computations (networking, fast storage access, and more)
- recent stabilization of GAT will enable further improvements in asynchronous features in the language. 
  This will empower the async ecosystem to experiment with new cool things.


### Encouraging security and sanity

The main selling point of rust - memory safety does not directly apply here: java/go is garbage collected and python uses references counting - they kind of are ok in this department.
But memory safety is not the only way we shoot ourselves in the foot on a daily basis. 
Rust's ownership model lends itself nicely to two essential topics in modern development: concurrency and parallelism. 

And if you haven't been paying attention there are major changes in the hardware industry going on. All big players are flirting with the smaller cores on the CPU.
[ARM](https://en.wikipedia.org/wiki/ARM_big.LITTLE), [Intel](https://arstechnica.com/gadgets/2021/11/intels-alder-lake-big-little-cpu-design-tested-its-a-barn-burner/), [Apple](https://developer.apple.com/news/?id=vk3m204o) all have them.
Even cloud vendors offer ARM on the server side like [AWS's graviton](https://aws.amazon.com/ec2/graviton/) or [Microsoft's one](https://azure.microsoft.com/en-us/blog/now-in-preview-azure-virtual-machines-with-ampere-altra-armbased-processors/).
Also at the RISC-V interest seems to hit the [sky](https://www.sifive.com/press/nasa-selects-sifive-and-makes-risc-v-the-go-to-ecosystem) too (sorry for the dad joke...).
It's way more cost-effective to use tools that can use the benefits of those platforms. 

Rust is statically typed and types like Result or Option are sprinkled everywhere in the standard library and the ecosystem with some nice safety measures.

TLDR for example rustc will get [quite mad](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-must_use-attribute) at you if you decide not to use the outcome of some calculation that might either finish with a successful result or not prevent the whole family of bugs.
But rust is not a solution to all the problems, you can opt out of some safety guarantees, and many times you will have to do something where rust cannot help you.
On a happy note though clippy - rust linter will also [scream](https://rust-lang.github.io/rust-clippy/master/#missing_safety_doc) at you if you decide not to document what are the safety rules that consumers of unsafe functions/methods must uphold.

## Finishing thoughts

Rust is here to stay and it is a perfect tool to have in your pocket both as an individual and organization. 
There are contexts where adopting rust is eased by possessing previous experience with similar technologies.
Sadly, in the context of the average web development shop, the cost of adopting rust for a new project is very high, even higher when it comes to rewriting a big chunk of codebases.
On the other hand, given certain constraints, it might prove enormous value to developers and the company itself.
I guess if you read to this point you wasted your time. There is no one answer :<


Topic: Difficult case of pitching rust - web development shop.

TLDR: Rust is a tough sell for JVM/python heavy companies with a lot of experience and actively maintained legacy code.
Sadly I don't think it's a good idea.

I've recently decided to collect my thoughts and arguments for pitching rust.
I tried to consider both sides - technology and business value.



Articles links to many big tech articles describing rust success stories (MS, AWS, Discord, Cloudflare).
During my research, I've found it hard to find some stories from not-so-small to medium-sized companies.
If you have some war stories or remarks I'd be very happy to include them in the article. 

