--- 
title: "Rust’s guiding principle"
date: 2024-11-26T21:58:34+01:00
draft: true
---

## Rust's guiding principle

I've been going through a fantastic book [Start with Why](https://www.amazon.com/Start-Why-Leaders-Inspire-Everyone/dp/1591846447) by Simon Sinek, and I couldn't shake the feeling that it reminded me of something. Then I recalled the talk **Inventing on principle** by Bret Victor.

{{< youtube PUv66718DII >}}


The main point of the talk is that, as a technologist, you can find your guiding principle.

> Something you believe is important and necessary and right, and using that to guide what you do

It is essential because it helps ruthlessly discard the unnecessary. It helps to stay laser-focused on the goal and make decisions aligned with it.

If we look at the rust's website, the guiding principle is clear.

> A language empowering everyone to build reliable and efficient software

It has occurred to me that that's the reason why rust might have resonated with me so much. It's not just a language, it's a guiding principle that aligns with mine. It helped me find my guiding principle.

> Leverage my passion for learning and technology to create reliable, efficient, high-quality solutions on time.

Rust as a language and community has empowered me to do just that. It has given me the tools and resources to gain confidence and stay curious.

**Contents:**

{{< toc >}}

## My personal experience

I would not call myself a technologist up until recently.

I finished my degree unconventionally - as an impulsive second to my primary finance degree at the same university. I've become a software developer almost by accident; I've never gone through hardships that many of you have gone through.

I recall reaching for The Book - rust's documentation about the language to check out the fuss.

The book clearly aims to intertwine the language's syntax with broader conceptual explanations in every chapter, starting from the very beginning

I was taught programming in 2015 using java and python, I never had to think about the the memory layout of the program. The hardware cost was already so low that it didn't matter if the program was inefficient. And there was always silent but uninspiring acceptance of the fact that the software, by definition, must be buggy.

At least in my experience with non-tech first companies, the industry's fast software delivery pace has made it easy to ignore the underlying concepts. Learning about those deeper, hidden layers never made sense because the language abstracted them away.

Following rules of thumb like *"don't block the actor"* or *"don't use global state"* was enough to write software that mostly worked.

Now I see the value of understanding the underlying concepts, I'm very grateful to the rust community for making the path to learning the language so easy.

## `impl Empower for Everyone`

To provide evidence of the guiding principle in action, below are some of the resources that have helped me learn rust.

### 📚 Books

#### The book

[The Book](https://doc.rust-lang.org/book/) is an excellent resource for learning rust. It's well-structured and easy to follow. It's a great example of how to write a dense book that is easy and fun to read.

The authors include: [Carol Nichols](https://github.com/carols10cents), [Steve Klabnik](https://github.com/steveklabnik), [Chris Krycho](https://github.com/chriskrycho) and more than 600 [other contributors](https://github.com/rust-lang/book/graphs/contributors).

#### Rust for rustaceans

Excellent [book](https://rust-for-rustaceans.com/) by [Jon Gjengset](https://thesquareplanet.com/) that dives deeper into the language. It covers topics in more detail and is commonly called the second book to read after The Book. Jon's way of explaining tricky concepts is insightful.


#### Rust in Action

Tim has written an amazing book [Rust in Action](https://www.manning.com/books/rust-in-action), focusing on practical problems. 

#### Asynchronous Programming in Rust

A much needed [book](https://www.packtpub.com/en-us/product/asynchronous-programming-in-rust-9781805128137) by [Carl Fredrik Samson](https://github.com/cfsamson). It's an excellent resource for learning about async programming in rust, a topic cited as one of the most difficult to grasp in rust next to lifetimes and ownership.


#### Zero to Production in Rust

A [book](https://zero2prod.com/) by [Luca Palmieri](https://www.lpalmieri.com/) that teaches you how to build an actual world application in rust.
It's an excellent resource for learning how to build a real-world application in rust if you prefer a more hands-on approach.


### Blogs

#### This Week in Rust

A [weekly newsletter](https://this-week-in-rust.org/) that keeps you up to date with the latest news in the rust community. It's a great way to stay up to date with the latest developments in the rust community.

It has served me as a great source of interesting blogs and projects to follow.

#### Rustlings

A great [resource](https://github.com/rust-lang/rustlings) for learning rust by solving small exercises. Another option is to use the [exercism](https://exercism.io/tracks/rust) track for rust or [Rust by Example](https://doc.rust-lang.org/rust-by-example/).

#### Common Rust Lifetime Misconceptions

A [blog post](https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md) by [pretzelhammer](https://github.com/pretzelhammer) that explains common misconceptions about lifetimes in rust as the title suggests. It's a great resource to check one's understanding of lifetimes in rust - a topic that is often cited as one of the most difficult to learn in rust.


### 🎧 Podcasts

#### The rustacean Station

A [podcast](https://rustacean-station.org/) created by community members that covers a wide range of topics in the rust community.
It's a great way to keep up with interesting projects (interviews with the creators) and learn about the latest developments in the rust community (deep dives into specific releases).

#### Rust in Production

Excellent podcast by [Matthias Endler](https://corrode.dev/podcast/) covers the practical aspects of using rust in production. It's a fantastic resource for learning how other companies are using rust in anger.


#### Self-Directed Research Podcast 

A [podcast](https://sdr-podcast.com/) by Amos or James that covers a wide range of topics in the rust community. Amos is also the author of [blog](https://fasterthanli.me/), which covers interesting topics around rust and networking and is written in a fun and engaging way.

The dynamic between the two hosts is very entertaining, and the topics are very interesting.


#### New Rustacean

A [podcast](https://newrustacean.com/) that covers Chris Krycho's journey learning rust. It's a bit older but still an excellent resource for learning rust for people who prefer podcasts over reading.

#### Oxide and Friends

A [podcast](https://oxide.computer/podcasts/oxide-and-friends) about the Oxide - company banking on rust to create a new generation of computers.
It's a great resource to listen to the practical aspects of using rust in a company from industry veterans who suffered from the problems rust is trying to solve.

### 🎥 Videos

#### Conferences

Rust has a lot of conferences. You can watch the talks on multiple channels:
- [RustConf 2024](https://www.youtube.com/playlist?list=PL2b0df3jKKiTWZeF7cip6ZUsaVXxWioRi)
- [Rust Nation UK](https://www.youtube.com/channel/UCLksRXfBiEITZMUo2ssjSdA)
- [Euro Rust](https://www.youtube.com/@eurorust)

You can also find scheduled events [here](https://foundation.rust-lang.org/events/)

#### Jon Gjengset's youtube channel

Jon has a great [YouTube channel](https://www.youtube.com/c/JonGjengset) where he explains rust concepts in a fun and engaging way. He has different formats like multi-hour long live coding sessions showing in detail how to implement a specific problem in rust, mainly in the context of the concurrent programming or shorter videos explaining a particular concept.
His attitude towards teaching is inspiring and extremely helpful in learning how to think about problems in rust.


#### Tim McNamara's YouTube channel

A great [YouTube channel](https://www.youtube.com/c/timclicks) where Tim explains rust concepts in a fun and engaging way.

I've especially enjoyed his series of interviews with the creators as he focuses on the human aspect of the creators and their motivations.

## Conclusion and a thank you

It's evident that the rust community is aligned with the guiding principle of the language.
I want to thank the rust community for making it easy to learn the language and for providing such a great resource from which to learn.
You have empowered me to write reliable and efficient software in rust.


PS: I'm sorry if I've missed some great resources. Please let me know, and I'll add them to the list or fire a MR to the [repo](https://github.com/FlakM/blog/tree/master/blog-static/content)

