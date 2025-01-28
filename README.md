[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**rskill** is clone of [npkill](https://github.com/voidcosmos/npkill), written in Rust for study purposes. It allows you to quickly find and delete `node_modules` directories to free up disk space. While it tries to follow the same CLI API as `npkill`, it does not implement all of it's features. **User discretion is advised** when deleting directories.

---

## Features

- 🚀 **Fast**: Slightly faster than the original `npkill` (though not streamed directly to the terminal and some features are missing such as automatic updates, bg color customization etc.).
- 🎯 **Same CLI API**: Supports the same command-line flags as `npkill` for familiarity.
- 🛠 **Study Project**: Written in Rust as a learning exercise.

---

## Options

| ARGUMENT                         | DESCRIPTION                                                                                                                                    |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| -d, --directory                  | Set the directory from which to begin searching. By default, starting-point is .                                                               |
| -E, --exclude                    | Exclude directories from search (directory list must be inside double quotes "", each directory separated by ',' ) Example: "ignore1, ignore2" |
| -f, --full                       | Start searching from the home of the user (example: "/home/user" in linux)                                                                     |
| --gb                              | Show folders in Gigabytes instead of Megabytes.                                                                                                |
| -h, --help, ?                    | Show this help page and exit                                                                                                                   |
| -s, --sort                       | Sort results by: `size`, `path` or `last-mod`                                                                                                  |
| -t, --target                     | Specify the name of the directories you want to search (by default, is node_modules)                                                           |
| -x, --exclude-hidden-directories | Exclude hidden directories ("dot" directories) from search.                                                                                    |
| -V, --version                    | Show rskill version                                                                                                                            |


## Installation

Download the latest release from [Releases](https://github.com/xsadia/rskill/releases) and add the binary to your system's PATH.
```
wget https://github.com/xsadia/rskill/releases/download/v0.1.0/rskill-linux-x86_64.tar.gz
tar -xzf rskill-linux-x86_64.tar.gz
sudo mv rskill /usr/local/bin
```

## Acknowledgments
  - Inspired by [npkill](https://github.com/voidcosmos/npkill).
