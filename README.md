# Dagan Utils (The ones written in Rust)

See <https://github.com/Property404/dagan-utils> for Python and shell scripts

## Line

Show specific lines in a file:

```
$ seq 1 10 | line 5
5
$ seq 1 10 | line 2..4
2
3
$ seq 1 10 | line 2..=4
2
3
4
$ seq 1 10 | line 5,8
5
8
```
