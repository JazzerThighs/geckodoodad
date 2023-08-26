Beginner project for learning Rust: geckodoodad

*Goal: Extract all of the gecko codes from a text file and compile them, adding categorization based on mem-address references.

Regular Expressions curtesy of @ribbanya:
https://github.com/MeleeWorkshop/wiki2gecko <= Being used as a general guide

Inside ```fn extract_opcode_and_address```, there is a string of OpCodes that the parser is searching for. This can be amended, or even replaced with a ```match``` statement to iterate over injections, for example, if one wanted to have each subsequent injected address accounted for in the output.