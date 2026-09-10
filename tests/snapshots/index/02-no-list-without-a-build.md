# No list without a build

The list describes what was built, so a run that builds nothing writes none.
This one only says what the folders would be called, and the section below
records that the file the settings asked for is not there.

Which is the point of showing it whether or not it exists: a list that failed
to appear is a visible absence rather than a section that quietly is not
there.

## Recording 75

```json
{"opera": {"title": "Billy Budd", "composer": {"lastName": "Britten", "firstName": "Benjamin"}}}
```

## Library

```tree
Britten/findopera-75.txt
```

## Toml

```toml
template = '''{{opera.title}}'''
destination = "{destination}"
link = "copy"

[index]
file = "00 - What is in here.txt"
template = '''{{opera.title}} (findopera {{id}})'''
```

## Run

```console
$ findopera organize ./library --dry-run
```
