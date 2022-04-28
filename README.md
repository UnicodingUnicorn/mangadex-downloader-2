# mangadex-downloader-2

Some time ago, [MangaDex](https://mangadex.org) changed their API and their website, so my [old tool](https://github.com/UnicodingUnicorn/mangadex-downloader) probably doesn't work anymore. In any case, this one is better, faster, stronger. Downloads cover images too.

As it stands, I baked the recommended rate limits into the application itself, so all should be good when querying the site.

## Usage

This is a CLI tool. I programmed it in Rust, so to compile it you'll need to install the Rust toolchain. Sorry. On the bright side, the help should be better than the last one, and should be all you need to get started.

I'll get around to doing releases sometime.

## Ranges

The format string for specify volume/chapter ranges is as follows:

```
 <volume start>[:chapter start][-<volume end>[:chapter end]][,<volume start>[:chapter start][-<volume end>[:chapter end]]]...
```

For example, to download Volume 1, and from Volume 9 Chapter 55 to Volume 12 Chapter 68 of a manga:

```
1,9:55-12:68
```
