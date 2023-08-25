Beginner project for learning Rust: geckodoodad

*Goal: Extract all of the gecko codes from a text file and compile them, adding categorization based on mem-address references.

Regular Expressions curtesy of @ribbanya:
https://github.com/MeleeWorkshop/wiki2gecko <= Being used as a general guide

(python)
gecko_re = re.compile(
    r"^ \$(?P<header>.*?)(?: *\((?P<version>(?:Melee|SSBM)? *(?:PAL|NTSC)? *(?:v?\d\.\d\d)?)?\))?"
    r"(?: *\[(?P<authors>.*?)\])?[ \t:]*$"
    r"(?P<description>(?:\n \*(?:.*?)$)*)"
    r"(?P<hex>(?:$\n [\dA-Za-z]{8} [\dA-Za-z]{8}[ \t]*(?:#.*)?$)+)",
    flags=re.MULTILINE)

Versions:
NTSC(1.00)
NTSC(1.01)
NTSC(1.02)
KOR
PAL