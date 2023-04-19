---
title: "Provisioning resources"
date: 2023-04-07T14:55:28+02:00
draft: true

series: ["Breeding cobras"]

Summary: '
How to provision EC2 machine with valid DNS record
'
---


## Provisioning the machine

To host the code I'll need two things:

1. A DNS domain - `flakm.com`
2. A NixOs server that will host code

The server should be reachable for ssh (at least at the beginning) and have port 443 opened and valid TLS certificate.

Since I heavily believe in self documentation I have used terraform for preparing the deployment.
I want to be able to redo the same setup and easily apply modifications without forgeting about one of the manual command.
I've also picked up a set of paid services and technologies that are rather certain to stay the same and are already well known by the community.
My requrements were following: 

- There should be already some tutorials that I can start from
- The whole infrastructure should be provisioned by single terraform and the state should be held in cloud
- The solution should be fully automated

## Prerequisites

I've used the amazing wiki article [Deploying NixOS using Terraform](https://nix.dev/tutorials/deploying-nixos-using-terraform) as base even though it's somehow dated.
To use the same configuration you will need following things:

1. [Terraform cloud account](https://app.terraform.io/)
2. [AWS credentials](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#authentication) with profile blog
3. [Cloudflare account](https://www.cloudflare.com/) 
4. DNS domain

It's absolutelly possible to deploy fully functioning system on your own host if you have a static IP.

## Setup


I'm using a [yubikey smart card with gpg agent for ssh authentication](https://github.com/drduh/YubiKey-Guide#configure-smartcard) and my public key is located in `~/.ssh/id_rsa_yubikey.pub`.
It's a very comfortable way because the key never touched the disk on my computers. But there is not a problem if you don't have one you might just use plain ssh keys:

```bash
# full resources: https://developers.yubico.com/PIV/Guides/Generating_keys_using_OpenSSL.html
openssl genrsa -out key.pem 2048
# extract public part
openssl rsa -in key.pem -outform PEM -pubout -out public.pem
```

Once you have all the other prerequisites setup you will need following setup:

```bash
# you can use alternative ways of providing access to aws credentials, especcially in CI/CD
aws sso login --profile blog


export CLOUDFLARE_API_TOKEN="..." # account token
export TF_VAR_ZONE_ID="..." # dns zone id
```

## Provisioning definition

The complete repository is hosted on github: https://github.com/FlakM/blog_deployment. Here is the complete terraform deployment definition:

```tf
terraform {
  cloud {
    organization = "flakm_mega_corp"

    workspaces {
      name = "blog"
    }
  }
  required_providers {
    cloudflare = {
      source = "cloudflare/cloudflare"
      version = "~> 3.0"
    }
  }
}

# this is working with aws sso 
# https://discuss.hashicorp.com/t/using-credential-created-by-aws-sso-for-terraform/23075/5
provider "aws" {
    profile = "blog"
    region = "eu-north-1"
}

module "nixos_image" {
    # the url is different since the tutorial one is old and has not been updated
    source  = "git::https://github.com/antoinerg/terraform-nixos.git//aws_image_nixos?ref=bcbddcb246f8d5b2ae879bf101154b74a78b6bc4"
    release = "22.11"
}

resource "aws_security_group" "ssh_and_egress" {
    # ssh
    ingress {
        from_port   = 22
        to_port     = 22
        protocol    = "tcp"
        cidr_blocks = [ "0.0.0.0/0" ]
    }
   
    # TLS for nginx
    ingress {
        from_port   = 443
        to_port     = 443
        protocol    = "tcp"
        cidr_blocks = [ "0.0.0.0/0" ]
    }

    # port 80 is required for ACMA challenge
    ingress {
        from_port   = 80
        to_port     = 80
        protocol    = "tcp"
        cidr_blocks = [ "0.0.0.0/0" ]
    }

    egress {
        from_port       = 0
        to_port         = 0
        protocol        = "-1"
        cidr_blocks     = ["0.0.0.0/0"]
    }
}


resource "aws_key_pair" "existing_key" {
  key_name   = "yubikey"
  public_key = file("~/.ssh/id_rsa_yubikey.pub")
}

resource "aws_instance" "machine" {
    ami             = module.nixos_image.ami
    instance_type   = "t3.micro"
    security_groups = [ aws_security_group.ssh_and_egress.name ]
    key_name        = aws_key_pair.existing_key.key_name

    root_block_device {
        volume_size = 50 # GiB
    }
}

output "public_ip" {
    value = aws_instance.machine.public_ip
}


provider "cloudflare" {
  # this is taken from CLOUDFLARE_API_TOKEN env 
}

variable "ZONE_ID" {
}

variable "domain" {
  default = "flakm.com"
}

resource "cloudflare_record" "blog" {
  zone_id = var.ZONE_ID
  name    = "@"
  value   = aws_instance.machine.public_ip
  type    = "A"
  proxied = false
}
```

To run it you will have to setup your environment. The flake in project's repository contains `devShell` section that provides the binaries required to run it.
If you have [direnv integration](https://nixos.wiki/wiki/Flakes#Direnv_integration) setup you will just need to run following command:

```bash
direnv allow
```

And your shell will magically receive all the required binaries (aws cli and terraform).
To deploy the code run:


```bash
terraform init # downloads the providers
terraform apply
```

At this point after couple of minutes you should be able to check the DNS adress.
It should point to valid ip address of the machine that you have just created:

```bash
nslookup flakm.com
```

If it does you should be able to login to the machine using ssh: 

```bash
ssh root@flakm.com
```

## Results

At this point I've managed to create following resources:

1. EC2 instance with public IP with 50 GiB disk and my public key that gives root access and NixOs
2. DNS A record that points to our instance
3. Security groups that open following ports: 22, 80 and 443 

We are ready to use nix!
