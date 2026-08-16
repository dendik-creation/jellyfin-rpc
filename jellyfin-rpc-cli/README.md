# Jellyfin-RPC
[![License](https://img.shields.io/github/license/JustRadical/jellyfin-rpc?color=purple)](https://www.gnu.org/licenses/gpl-3.0-standalone.html)
[![Crates.io](https://img.shields.io/crates/v/jellyfin-rpc-cli.svg)](https://crates.io/crates/jellyfin-rpc-cli)
[![Downloads](https://shields.io/github/downloads/JustRadical/jellyfin-rpc/total)](https://github.com/JustRadical/jellyfin-rpc/releases/latest)
[![Visitors](https://visitor-badge.laobi.icu/badge?page_id=radiicall.jellyfin-rpc)](https://github.com/JustRadical/jellyfin-rpc)

[Frequently Asked Questions](https://github.com/JustRadical/jellyfin-rpc/wiki/Frequently-Asked-Questions)

Program used to display what you're currently watching on discord.

Jellyfin-RPC uses the API to check what you're currently watching, this means that the program can be ran from a server or your computer. The only requirement is that discord is open and logged in.

## Install

For installation instructions refer to the [Wiki](https://github.com/JustRadical/jellyfin-rpc/wiki/Installation)

## Setup

Copy [`.env.example`](../.env.example) to `.env`, fill in the server url, API key and
username, and run `jellyfin-rpc`. Every option is documented inside that file.

A JSON config file still works and is still read first; see [`example.json`](../example.json).
For the original single-server setup guide refer to the
[Wiki](https://github.com/JustRadical/jellyfin-rpc/wiki/Setup).

### Which device does it show?

Any of them. Jellyfin sessions are matched by **username**, not by device, so
watching from a phone, a browser or the desktop app all produce the same
presence. The only machine that matters is the one running `jellyfin-rpc`:
Discord's IPC socket is local, so the binary has to run next to the Discord
client you want the status to appear on.

### Watching several servers at once

Servers are polled in the order they are configured, and the first one with
something playing wins. A server that is offline is logged and skipped, so a
remote server going down never stops the local one from being read.

```dotenv
JELLYFIN_NAME=vps
JELLYFIN_URL=https://jellyfin.example.com
JELLYFIN_API_KEY=...
JELLYFIN_USERNAME=me

JELLYFIN_2_NAME=local
JELLYFIN_2_URL=http://localhost:8096
JELLYFIN_2_API_KEY=...
```

`JELLYFIN_2_*` through `JELLYFIN_9_*` add further servers. `JELLYFIN_USERNAME`
applies to every server; give a server its own `JELLYFIN_n_USERNAME` only when
the account name differs there.

### Artwork from a server that is not public

Discord loads the large image itself, so it must be able to reach the URL.
`IMAGES_HOSTING=direct` therefore cannot work for `http://localhost:8096` — use
`imgur` (needs `IMGUR_CLIENT_ID`) or `litterbox` (no account, links last 72h),
which upload the poster and hand Discord a public link. Uploads are cached per
item in `urls.json`, so each poster is uploaded once.

### Where configuration comes from

Later layers override earlier ones:

1. built-in defaults
2. the JSON config file — `%APPDATA%\jellyfin-rpc\main.json`, or `-c <path>`
3. environment variables, seeded from the first `.env` found in the working
   directory, next to the binary, then the config directory (`-e <path>` to pick one)
4. command line flags

Run `jellyfin-rpc --print-config` to see what all of that resolved to, and
`-v debug` to log which server and which device each session came from.

### Changing the name Discord shows

The word after "Watching" is the name of the Discord *application* behind
`DISCORD_APPLICATION_ID`. To change it, create an application at
[discord.com/developers](https://discord.com/developers/applications), name it
whatever you want, and put its id in `DISCORD_APPLICATION_ID`.


## Pictures of Jellyfin-RPC in action

#### Movie

<img alt="Jellyfin-RPC Displaying a Movie in Discord" src="https://github.com/user-attachments/assets/b4663372-e145-414c-82e1-b4343512da08" />

#### Episode

<img alt="Jellyfin-RPC Displaying a TV Show Episode in Discord" src="https://github.com/user-attachments/assets/5ae1db9f-8897-4340-b660-a998195a26b8" />


#### Music

<img alt="Jellyfin-RPC Displaying a Song in Discord" src="https://github.com/user-attachments/assets/b5963c01-37ed-4c4f-bff3-c0310a11dc14" />

#### Live TV

###### Note: does not look like this anymore, no longer have this set up to test
![Jellyfin-RPC Displaying a TV Channel in Discord](https://github.com/JustRadical/jellyfin-rpc/assets/66682497/1d9cf0af-96f2-438b-b147-904ab65bcc48)

#### Book

<img alt="Jellyfin-RPC Displaying a Book in Discord" src="https://github.com/user-attachments/assets/bd7b70ab-a4ee-4f3b-8437-a500e33441c5" />

#### Audiobook

###### Note: does not look like this anymore, no longer have this set up to test
![Jellyfin-RPC Displaying an Audiobook in Discord](https://github.com/JustRadical/jellyfin-rpc/assets/66682497/3a7845ae-0219-4932-a1a2-efb44f40a171)

#### Terminal

<img width="847" height="246" alt="Image of terminal/cmd output" src="https://github.com/user-attachments/assets/cc603d06-784b-4d4b-b35b-04af006775bd" />

</details>

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=JustRadical/jellyfin-rpc&type=Date&theme=dark)](https://star-history.com/#JustRadical/jellyfin-rpc&Date&theme=dark)
