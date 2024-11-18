--- 
title: "Automating the pain away"
date: 2024-11-16T21:33:21+01:00
draft: false
authors: ["Maciej Flak"]
description: "
Some tasks just don't spark joy—they're repetitive, time-consuming, and often stand in the way of meaningful work. Transforming structured markdown notes into Jira tasks is one of those uninspiring chores. While planning, discussing, and iterating with the team is rewarding, manually creating Jira tickets can feel like trudging through UI quicksand.

This post delves into how automating these tasks can make your workflow more efficient and enjoyable. By leveraging command-line tools and the power of Large Language Models (LLMs), we explore how to automate Jira ticket creation from structured notes. Not only can this process reduce procrastination, but it also frees up time and energy for more creative pursuits. From scripting and refining bash automation to using LLMs for parsing and script generation, this approach shows that even the most frustrating tasks can be transformed into something seamless. Automate the tedious, embrace the creative, and unlock the true potential of streamlined productivity.
"
tags: ["jira", "cli", "linux", "llm", "automation", "productivity"] 
images: [
    "/images/cogs.jpg",
    ]
featured_image: "/images/cogs.jpg"
---

## Automate the unispiring tasks

Some tasks do not spark joy. They are repetitive, tedious, and time-consuming. But they need to be done 💀

A prime example of such a task is changing my markdown notes describing a project to jira entries.

I like the process of planning the tasks, discussing them with the team, and iterating on the plan. But with the most profound hatred, I don't like the process of creating the jira tickets. The UI is junky, the process is slow, and the whole experience is just a pain.

Maybe automate it?

But how do you pick the tasks that are worth automating?

There is some prior art in this area. The xkcd 1205 below captures the essence of the problem quite well:

{{< figure src="https://imgs.xkcd.com/comics/is_it_worth_the_time.png" title="Is it worth the time?" link="https://xkcd.com/1205/" >}}

In this post, I'll suggest two new dimensions for picking the tasks we automate due to the recent advancements in the LLMs.

1. Is the output of the task valuable?
2. How strongly do you procrastinate on the task?


The xkcd comic doesn't capture the idea that we can automate tasks *far more quickly* than before.
The LLMs suck, but they are good enough to suggest a first draft of automation.

They can also be very effective at getting unstack. Do you need to remember how to iterate over the elements in bash? Or even worse, did you never know that `jq` exists?

Putting apart the boiling ocean stuff and hoping that hardware advances will magically solve the problem, I suspect that the LLMs will become more and more self-hostable and energy-efficient.


## The jira automation

I have a personal habit of taking notes in [obsidian](https://obsidian.md/). It allows me to gather my thoughts and ideas and quickly find the misconceptions in my thinking.

Obsidian gives some structure but doesn't enforce too strict rules.

To give a concrete example, here is an imaginary note that I'd create about an upcoming task:

```markdown
# Deploy a new application X

## Description

We have a new application X that should serve as a replacement for the old application Y.
Here are the relevant links:

1. [ADR describing the requirements]()
2. [Link to the discussion about the new application]()


## Steps

1. Setup the new application
    - [ ] Create a new repository
    - [ ] Write the hello world application
    - [ ] Setup the CI/CD pipeline
2. Deploy the application
    - [ ] Modify the GitOps repository to describe the new application
    - [ ] Deploy the application using X pipeline
3. Add monitoring & tracing
    - [ ] Add the monitoring and tracing to the application
4. Write a POC for the new feature
    - [ ] Implement the basic version of the feature - mock the data
    - [ ] Prepare the demo for the stakeholders
5. Prepare connection to external services
    - [ ] Provision the necessary resources
        - [ ] s3 bucket
        - [ ] database
    - [ ] Setup the application to use the resources and health checks
6. Prepare the feature flags in the application Z
    - [ ] Add the feature flags to the application
    - [ ] Prepare the documentation for the feature flags
7. Route test traffic to the new application
```

At this time, I'd like to discuss with the team and quickly iterate on the steps.

When we agreed on the steps, the problem was that I needed to create the jira epic and the subtasks to fulfill the official dance.
Depending on the team's expertise and feature complexity, this document might differ quite a lot, but we need to fit it into the jira to track the progress.

Previously, I'd procrastinate like crazy until my manager or PM would scream at me to create the jira ticket. The vision of touching the jira UI made the planning part of the task unattainable.

Once I've automated the process, I've discovered that I'm more willing to plan the tasks and iterate on them.

## The automation steps

The automation steps are trivial. First, I've looked for a cli tool that would allow me to create the jira tickets. 

I've found the [`jira-cli`](https://github.com/ankitpokhrel/jira-cli) tool to create the tickets from the command line.

The second part was understanding how to use it, which was also relatively easy - not because the tool is well-documented but because I've used chatty jeeps.

Previously, it would have taken a couple of hours to understand the tool, but given the low stakes, I've quickly iterated on the script. And came up with the following:

```bash
# create the the issue
jira issue create \
    --label "rust" \
    --summary "Summary of the task" \
    --type Task \
    --body "Longer form body
    ### It even uses markdown structure" \
    --custom story-points=3 \
    --no-input >> output.log

# add the issue to the epic
jira epic add EPIC_ID ISSUE_ID
```

Getting the proper configuration for custom fields required some fiddling, but once I got it, it was a breeze.

## The trick - LLMs

The trick was to use LLMs again to parse the markdown file and create the jira tickets.

I've provided the context in the web ui: the command line to create the tickets and my preferred task structure, and asked the model to generate the bash script.

Once I've got the script, I've iterated it to fix the issues quickly, like dumb action points or invalid structure.

## Taking the automation further

The automation could be better. It requires me to run the script and manually gather the output.
It can be further automated by using cli tool [`chatgpt-cli`](https://github.com/j178/chatgpt) to interact with the LLM. Here is how it can be done:

```bash
# create the jira prompt and save it to the config
jira_prompt="$(jq -R -s @json <<EOF
Create a bash script that creates the jira epic and tickets.
Based on markdown file with the tasks, and I'd like to get the script that will generate the jira tickets.

1. Create the epic describing the whole project from description and tasks
2. Parse the tasks from the markdown file (-[] checkboxes are action points) 
3. Create the jira tickets for each task every task separately using line breaking \ and multiline strings
4. Add the tickets to the epic

In the output:
- for ALL tasks add `echo` command with format: - [\$id](\$url) - \$name
- Strings with multiple lines should be multiline strings not \n separated for readability.
- Format for human readability. Be correct and concise.
- All tickets and epics must contain short summary and a longer description.
- All tickets MUST contain ## Acceptance Criteria section with - [ ] checkboxes representing action points.
- If the task seems small assign 1 story point, if it seems big assign 3 story points. If it seems huge assign 5 story points.
- ONLY include bash with shebang #!/usr/bin/env bash. Do not include any other code or comments or block using quotes or backticks.
- do not use for loop and arrays - repeat the code for each task

Here is how you create the jira ticket:

url="\$(jira issue create \
    --label "rust" \
    --summary "Summary of the task" \
    --type Task \
    --body "Longer form body
    ### It even uses markdown structure" \
    --custom story-points=3 \
    --no-input | grep -o 'https://[^ ]*$')"
id="\${url##*/}"


Here is how you create the epic:

epic_url="\$(jira epic create -n"Epic epic" -s"Everything" -yHigh -lbug -lurgent -b"Bug description" --no-input | grep -o 'https://[^ ]*$')"
epic_id="\${epic_url##*/}"

and here is how you add the issue to the epic:

jira epic add "\$epic_id" "\$id"

EOF
)"

# update the config to update the jira prompt
jq --arg jira_prompt "$jira_prompt" '.prompts.jira = $jira_prompt' ~/.config/chatgpt/config.json > ~/.config/chatgpt/config_tmp.json && mv ~/.config/chatgpt/config_tmp.json ~/.config/chatgpt/config.json

# use 4o model it requires 1.3.5 version of the chatgpt-cli
jq '.conversation.model = "gpt-4o"' ~/.config/chatgpt/config.json > tmp.$$.json && mv tmp.$$.json ~/.config/chatgpt/config.json

# increase the max tokens to 4096
jq '.conversation.max_tokens = 4096' ~/.config/chatgpt/config.json > tmp.$$.json && mv tmp.$$.json ~/.config/chatgpt/config.json

```

And now we can generate the script by running the following command:

```bash
# either set the OPENAI_API_KEY in the environment or provide it in configuration
export OPENAI_API_KEY="your-api-key"

# generate the script based on the markdown file
cat application-x.md | chatgpt -n -p jira > jira_script.sh
```

We now have a script that will create the jira tickets for us! 🎉

I tend to edit the parts that are not perfect quickly. It's a quick process, and I'm happy with the results.

**⚠️ WARNING ⚠️**: Never run the script without reviewing it. It might contain malicious code or errors that could cause damage.

```bash
# run shellcheck to check for the common errors
shellcheck jira_script.sh

# finally after reviewing the script, execute it
chmod +x jira_script.sh
./jira_script.sh
```

Here is the top lines of the generated script:

```bash
#!/usr/bin/env bash

epic_url="$(jira epic create -n "New Application X Deployment" -s "Epic for the replacement of old application Y with new application X" -y High -l "feature" -l "urgent" -b "We need to migrate to application X to enhance our service offerings. Please refer to the ADR for detailed requirements." --no-input | grep -o 'https://[^ ]*$')"
epic_id="${epic_url##*/}"

url="$(jira issue create --label "task" --summary "Setup the new application" --type Task --body "## Setup the new application

We need to setup the new application X in a new repository. The initial requirement is to create a 'hello world' application and setup a CI/CD pipeline.

### Acceptance Criteria
- [ ] Create a new repository
- [ ] Write the hello world application
- [ ] Setup the CI/CD pipeline" --custom story-points=3 --no-input | grep -o 'https://[^ ]*$')"
id="${url##*/}"
jira epic add "$epic_id" "$id"
echo "- [$id]($url) - Setup the new application"
```


## Do more by doing less

Automating the uninspiring tasks that block the creative process is far more important than the time saved. By spending our focus on the creative process, we can achieve more and be happier with the results.

The broader lesson is that automating small, high-friction tasks can significantly affect how we approach work.

No more procrastinating over clicking through sluggish UIs or dreading repetitive data entry; instead, we can focus on designing the system once and letting it do the rest.
