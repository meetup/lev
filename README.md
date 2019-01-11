# lev [![Build Status](https://travis-ci.org/meetup/lev.svg?branch=master)](https://travis-ci.org/meetup/lev) [![Coverage Status](https://coveralls.io/repos/github/meetup/lev/badge.svg?branch=master)](https://coveralls.io/github/meetup/lev?branch=master)

> AWS lambda env manager

# 📦 install

## Via github releases

Prebuilt binaries for osx and linux are available for download directly from [Github Releases](https://github.com/meetup/lev/releases)

```bash
$ curl -L \
 "https://github.com/meetup/lev/releases/download/v0.0.0/lev-v0.0.0-$(uname -s)-$(uname -m).tar.gz" \
  | tar -xz
```

## Usage

Lev uses the AWS default credential chain to authenticate requests with AWS apis.

```bash
AWS_PROFILE=prod lev
lev 0.1.0
softprops <d.tangren@gmail.com>
AWS lambda env manager

USAGE:
    lev <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    get      Gets a function's current env
    help     Prints this message or the help of the given subcommand(s)
    set      Sets a function's env var
    unset    Unsets a function's env var
```