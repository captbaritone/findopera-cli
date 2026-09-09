# A lifespan is both years or just the birth year

Handel has both years; John Adams is alive, so only a birth year is known. The
conductors have neither, and the group takes the whole clause away — which is
why a lone death year is left out rather than spelled somehow: there is
nowhere in a filename for a question mark, and `-1602` reads as a negative
number.

## Library

```tree
sosarme/findopera-10655.txt
nixon/findopera-9154.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{composer.lastName}}[ ({{composer.dates}})]/{{opera.title}}[ - {{conductor.lastName}} ({{conductor.dates}})]'
```
