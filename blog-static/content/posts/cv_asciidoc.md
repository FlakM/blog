+++ 
title =  "Writing a CV in AsciiDoc"
date = 2024-10-05T13:12:46+02:00
draft = true
description = """
How to write a pretty CV in AsciiDoc. A story about ditching google docs and embracing the power of plain text.
"""
tags = ["asciidoc", "cv", "resume"]
categories = []
mastodonName = "@flakm@hachyderm.io"
+++


## Introduction

I've historically used Google Docs for my CV. It's a great tool, but I've been looking for a way to write my CV in plain text. I've reaserched a few options and decided to go with AsciiDoc. 

## Why AsciiDoc?

AsciiDoc is a human-readable document format that is both simple and powerful. It's a plain text format that can be converted to HTML, PDF, and other formats.

I've been mostly using markdown for my writing and documentation since tools like obsidian have defaulted to it. But for a CV I wanted something more powerful - especially when it comes to formatting.

After googling around I've found a few examples already ready to use. The one that caught my eye was [asciidoctor-web-pdf](https://github.com/ggrossetie/asciidoctor-web-pdf). Sadly it was not packaged for nixos, so I've decided to test using docker.

## Setting up the environment

I've copied the [example](https://github.com/ggrossetie/asciidoctor-web-pdf/tree/main/examples/resume) from the repository and switched some images around.
After that I've created a simple bash script to run the docker container:

```bash
#!/usr/bin/env bash
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

docker run -i --rm \
  --volume=$DIR/:"/usr/app" \
  -u $(id -u ${USER}):$(id -g ${USER}) \
  ggrossetie/asciidoctor-web-pdf:latest \
  --template-require ./template.js resume.adoc

mv $DIR/resume.pdf $DIR/../blog-static/static/documents/resume.pdf
rm $DIR/resume.html
```

It uses the template and resources from the example and moves the generated pdf to the static folder of my blog.
If you are interested about the blog setup you can check the [series about blog deployment](/series/simple-personal-blog/).


Since the blog is deployed using nixos I can just run:

```bash
nixos-rebuild switch --target-host root@hetzner-blog --flake .#blog
```

and the new CV will be available on the internet under path [/documents/resume.pdf](/documents/resume.pdf) 🎉

The results are quite nice and I'm happy with the switch. I can now easily version control my CV and generate it in multiple formats.
I can also easily share it with others by just sending them a link to the pdf.

{{< figure src="/images/cv.png" class="img-sm" caption="Printscreen of the generated CV available [here](/documents/resume.pdf)" >}}


The source code for the CV is available [here]().

