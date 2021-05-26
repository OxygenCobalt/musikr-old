# musikr

musikr is a command-line music tagger with sensible defaults and an intuitive user experience. [Work In Progress]

### Current Plan

```
Planned tag formats:

ID3 [In-Progress]
Vorbis [Will be done]
MP4 [Will be done]

Planned Commands:

Usage: musikr [FLAGS] [tag(=value)...] [FILES...]

Manipulate the metadata of music files

Singular Flags:
-a --add [tag=value]             Add a metadata tag
-d --delete [tags]               Delete a metadata tag
-m --modify [tag=value]          Modify existing tags
-M --modadd [tag=value]          Modify existing tags or add new tags
-o --output [tags]               Print specified tags
-O --outraw [tags]               Print raw binary data for specified tags
-0 --outall [tags]               Print raw binary and header data for a tag
-u --upgrade [files...]          Upgrade files to the latest version of their tag format
-c --copy [tags] [src] [dest...] Copy tags from the source file to the destination files
-h --help                        Output this message

Modifier flags:
-r --recursive  Be recursive
-q --quiet      Don't warn about obsolete metadata, destructive operations, etc.
-p --pedantic   Output all technical information

Planned Structure:
musikr - Tagging CLI tool
libmusikr - Library for tag reading and writing
```
