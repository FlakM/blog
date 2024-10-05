+++ 
title =  "Writing a CV in AsciiDoc and ditching Google Docs"
date = 2024-10-05T13:12:46+02:00
draft = false
description = """
How do you write a pretty CV using AsciiDoc? It is a story about ditching Google Docs and embracing the power of plain text.
"""
tags = ["asciidoc", "cv", "resume"]
categories = []
mastodonName = "@flakm@hachyderm.io"
+++

## Writing a CV, a wrong way

I've historically used Google Docs for my CV. It's a great tool, but I've had some issues with it for my use case:

- Styling was never consistent, and changing styles was difficult
- Since I'm not a native English speaker and I'm a dyslexic, I had to keep going between sites like Grammarly and docs - which messed with the formatting (and I refuse to have it installed as a browser extension)
- I've spent sooo fucking much time perfecting nvim configuration and building muscle memory that it felt like a waste not to use it everywhere.
- It never felt fun

After having a blast with markdown for documentation and personal notes, I've been looking for a way to write my CV in plain text. I've researched a few options and decided to go with AsciiDoc. 

## AsciiDoc, a better way?

AsciiDoc is a human-readable document format that is both simple and powerful. It's a plain text format that can be converted to HTML, PDF, and other formats like markdown. But it seems so much richer; it's wild.

After googling around, I found some examples that were already ready to use. The one that caught my eye was [asciidoctor-web-pdf](https://github.com/ggrossetie/asciidoctor-web-pdf). Sadly, it was not packaged for nixos, so I've decided to test using docker.

## Hammer time! 🔨

I've copied the [example](https://github.com/ggrossetie/asciidoctor-web-pdf/tree/main/examples/resume) from the repository, uploaded my own images, and created a script to run the docker container:

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

It uses the template and resources from the example and moves the generated PDF to the static folder of my blog 🪄.

Since the blog is deployed using nixos, I can just run the:

```bash
nixos-rebuild switch --target-host root@hetzner-blog --flake .#blog
```

and the new CV will be available on the internet under path [/documents/resume.pdf](/documents/resume.pdf) 🎉. Check the [series about blog deployment](/series/simple-personal-blog/) to learn more about the blog setup.

The results are satisfying, and I'm happy with the switch for a few reasons:

- The source code is under version control
- It's much easier to paste it into tools like grammarly
- I can tweak the look with CSS <3
- I can easily share it with others by sending them a link to the pdf


{{< figure src="/images/cv.png" class="img-sm" caption="Printscreen of the generated CV available [here](/documents/resume.pdf)" >}}


You can copy the source code for the CV from [here](https://github.com/FlakM/blog/tree/master/cv) and have a brand new CV in no time.

{{< figure src="/images/oprah_you_get_a_cv.jpg" class="img-sm" >}}

## Future improvements

It would be awesome to package `asciidoctor-web-pdf` for nixos and use it directly without docker. I might look into that in the future.
