Beginner project for learning Rust: geckodoodad

*Goal: Extract all of the gecko codes from a text file and compile them, adding categorization based on mem-address references.

Regular Expressions curtesy of @ribbanya:
https://github.com/MeleeWorkshop/wiki2gecko <= Being used as a general guide

Inside ```fn extract_opcode_and_address```, there is a ```match``` statement to perform different writing operations based on the OpCode of a 4-Byte word. 