[![Release](https://img.shields.io/github/v/tag/scotow/ovr?label=version)](https://github.com/scotow/ovr/tags)
[![Build Status](https://img.shields.io/github/actions/workflow/status/scotow/ovr/docker.yml)](https://github.com/scotow/ovr/actions)


![Banner](banner.png)

## Features

- JSON, Text (with human readable), HTML + CSS
- `/today` and `/next`
- `/find?dish=YOUR_FAVORITE_DISH`
- [iCalendar](https://icalendar.org)

## Upload

```bash
# Upload all pdf in the current directory:
$ ls *.pdf | xargs printf -- '-F file=@%s\n' | xargs curl -v localhost:8080
```

## Docker

```
docker run ghcr.io/scotow/ovr/api:latest
```